cmake_minimum_required(VERSION 3.28)
project(Workspace VERSION 1.2.3 DESCRIPTION "Example monorepo" LANGUAGES C CXX)

include_guard(GLOBAL)
include(cmake/CompilerWarnings.cmake OPTIONAL)

option(BUILD_TESTING "Enable tests" ON)
option(BUILD_EXAMPLES "Enable examples" OFF)

add_library(core STATIC src/core/a.cc src/core/b.cc)

target_include_directories(
  core
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
  PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src)

target_compile_definitions(core PUBLIC CORE_HAS_FEATURE=1)

target_link_libraries(core PUBLIC Threads::Threads PRIVATE project_warnings)

install(
  TARGETS core
  EXPORT WorkspaceTargets
  ARCHIVE
  DESTINATION lib
  LIBRARY
  DESTINATION lib
  RUNTIME
  DESTINATION bin)
