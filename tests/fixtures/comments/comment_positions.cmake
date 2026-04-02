# Leading file comment
cmake_minimum_required(VERSION 3.20)

# Comment before a command
project(MyProject)

message(STATUS "hello") # trailing comment on command

# Multiple
# consecutive
# comments

set(MY_VAR "value")

target_sources(cmfmt
  PRIVATE
    src/lib.rs # inline comment after arg
    #[[ bracket comment between args ]]
    src/main.rs
)

#[[ standalone bracket comment ]]

# Empty comment line:
#
# Above was empty
