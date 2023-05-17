
bool findUnwindSections(int_t targetAddr, UnwindInfoSections &info)
{
    // Use DLFO_STRUCT_HAS_EH_DBASE to determine the existence of
    // `_dl_find_object`. Use _LIBUNWIND_SUPPORT_DWARF_INDEX, because libunwind
    // support for _dl_find_object on other unwind formats is not implemented,
    // yet.
#if defined(DLFO_STRUCT_HAS_EH_DBASE) & defined(_LIBUNWIND_SUPPORT_DWARF_INDEX)
    // We expect `_dl_find_object` to return PT_GNU_EH_FRAME.
#if DLFO_EH_SEGMENT_TYPE != PT_GNU_EH_FRAME
#error _dl_find_object retrieves an unexpected section type
#endif
    // We look-up `dl_find_object` dynamically at runtime to ensure backwards
    // compatibility with earlier version of glibc not yet providing it. On older
    // systems, we gracefully fallback to `dl_iterate_phdr`. Cache the pointer
    // so we only look it up once. Do manual lock to avoid _cxa_guard_acquire.
    static decltype(_dl_find_object) *dlFindObject;
    static bool dlFindObjectChecked = false;
    if (!dlFindObjectChecked)
    {
        dlFindObject = reinterpret_cast<decltype(_dl_find_object) *>(
            dlsym(RTLD_DEFAULT, "_dl_find_object"));
        dlFindObjectChecked = true;
    }
    // Try to find the unwind info using `dl_find_object`
    dl_find_object findResult;
    if (dlFindObject && dlFindObject((void *)targetAddr, &findResult) == 0)
    {
        if (findResult.dlfo_eh_frame == nullptr)
        {
            // Found an entry for `targetAddr`, but there is no unwind info.
            return false;
        }
        info.dso_base = reinterpret_cast<uintptr_t>(findResult.dlfo_map_start);
        info.text_segment_length = static_cast<size_t>(
            (char *)findResult.dlfo_map_end - (char *)findResult.dlfo_map_start);

        // Record the start of PT_GNU_EH_FRAME.
        info.dwarf_index_section =
            reinterpret_cast<uintptr_t>(findResult.dlfo_eh_frame);
        // `_dl_find_object` does not give us the size of PT_GNU_EH_FRAME.
        // Setting length to `SIZE_MAX` effectively disables all range checks.
        info.dwarf_index_section_length = SIZE_MAX;
        EHHeaderParser<LocalAddressSpace>::EHHeaderInfo hdrInfo;
        if (!EHHeaderParser<LocalAddressSpace>::decodeEHHdr(
                *this, info.dwarf_index_section, info.dwarf_index_section_length,
                hdrInfo))
        {
            return false;
        }
        // Record the start of the FDE and use SIZE_MAX to indicate that we do
        // not know the end address.
        info.dwarf_section = hdrInfo.eh_frame_ptr;
        info.dwarf_section_length = SIZE_MAX;
        return true;
    }
#endif
    dl_iterate_cb_data cb_data = {this, &info, targetAddr};
    int found = dl_iterate_phdr(findUnwindSectionsByPhdr, &cb_data);
    return static_cast<bool>(found);
}
