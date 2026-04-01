# leading file comment
message(STATUS "boot") # trailing comment

target_sources(cmfmt
  PRIVATE
    src/lib.rs # keep this inline
    #[[ bracket comment between args ]]
    src/main.rs
)
