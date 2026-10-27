function(f target)
    target_sources("${target}" PUBLIC
        FILE_SET HEADERS
        BASE_DIRS "${ARG_PUBLIC_HEADER_BASE_DIR}"
        FILES ${ARG_PUBLIC_HEADERS})
endfunction()
