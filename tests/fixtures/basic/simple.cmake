cmake_minimum_required(VERSION 3.28)
project(cmfmt LANGUAGES C CXX)

add_library(cmfmt src/lib.rs)
target_link_libraries(cmfmt PUBLIC fmt::fmt PRIVATE helper)
