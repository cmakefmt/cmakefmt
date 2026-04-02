# Minimum CMake required. If available, accept the policy-controlled behavior up
# to 3.26.
cmake_minimum_required(VERSION 3.16...3.26)

# Revert to old behavior for MSVC debug symbols.
if(POLICY CMP0141)
  cmake_policy(SET CMP0141 OLD)
endif()

if(protobuf_VERBOSE)
  message(STATUS "Protocol Buffers Configuring...")
endif()

# Project
project(protobuf C CXX)

if(CMAKE_CXX_STANDARD AND CMAKE_CXX_STANDARD LESS 17)
  message(FATAL_ERROR "The minimum supported C++ standard is C++ 17")
endif()

if(protobuf_DEPRECATED_CMAKE_SUBDIRECTORY_USAGE)
  if(CMAKE_PROJECT_NAME STREQUAL "protobuf")
    get_filename_component(CMAKE_SOURCE_DIR ${CMAKE_SOURCE_DIR} DIRECTORY)
  endif()
  get_filename_component(CMAKE_CURRENT_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR} DIRECTORY)
  get_filename_component(PROJECT_SOURCE_DIR ${PROJECT_SOURCE_DIR} DIRECTORY)
  get_filename_component(protobuf_SOURCE_DIR ${protobuf_SOURCE_DIR} DIRECTORY)
endif()

# Options
option(protobuf_INSTALL "Install protobuf binaries and files" ON)
option(protobuf_BUILD_TESTS "Build tests" OFF)
option(protobuf_BUILD_CONFORMANCE "Build conformance tests" OFF)
option(protobuf_BUILD_EXAMPLES "Build examples" OFF)
option(protobuf_BUILD_PROTOBUF_BINARIES "Build protobuf libraries and protoc compiler" ON)
option(protobuf_BUILD_PROTOC_BINARIES "Build libprotoc and protoc compiler" ON)
option(protobuf_BUILD_LIBPROTOC "Build libprotoc" OFF)
option(protobuf_BUILD_LIBUPB "Build libupb" ON)
option(protobuf_DISABLE_RTTI "Remove runtime type information in the binaries" OFF)
option(protobuf_TEST_XML_OUTDIR "Output directory for XML logs from tests." "")
option(protobuf_ALLOW_CCACHE "Adjust build flags to allow for ccache support." OFF)
option(protobuf_FORCE_FETCH_DEPENDENCIES "Force all dependencies to be downloaded from GitHub.  Local installations will be ignored." OFF)
option(protobuf_LOCAL_DEPENDENCIES_ONLY "Prevent downloading any dependencies from GitHub. If this option is set, the dependency must be available locally as an installed package." OFF)

# We support Unity (Jumbo) builds best-effort.
option(protobuf_USE_UNITY_BUILD "Enable Unity (Jumbo) build for" OFF)
if (BUILD_SHARED_LIBS)
  set(protobuf_BUILD_SHARED_LIBS_DEFAULT ON)
else (BUILD_SHARED_LIBS)
  set(protobuf_BUILD_SHARED_LIBS_DEFAULT OFF)
endif (BUILD_SHARED_LIBS)
option(protobuf_BUILD_SHARED_LIBS "Build Shared Libraries" ${protobuf_BUILD_SHARED_LIBS_DEFAULT})
include(CMakeDependentOption)
cmake_dependent_option(protobuf_MSVC_STATIC_RUNTIME "Link static runtime libraries" ON
  "NOT protobuf_BUILD_SHARED_LIBS" OFF)
set(protobuf_WITH_ZLIB_DEFAULT ON)
option(protobuf_WITH_ZLIB "Build with zlib support" ${protobuf_WITH_ZLIB_DEFAULT})
set(protobuf_DEBUG_POSTFIX "d"
  CACHE STRING "Default debug postfix")
mark_as_advanced(protobuf_DEBUG_POSTFIX)

if(WITH_PROTOC)
  set(protobuf_PROTOC_EXE protoc)
  set(protobuf_BUILD_PROTOC_BINARIES OFF)
  add_executable(protoc IMPORTED GLOBAL)
  add_executable(protobuf::protoc ALIAS protoc)
  set_property(TARGET protoc PROPERTY IMPORTED_LOCATION ${WITH_PROTOC})
endif()

# User options
include(${protobuf_SOURCE_DIR}/cmake/protobuf-options.cmake)

if (protobuf_BUILD_SHARED_LIBS)
  set(CMAKE_POSITION_INDEPENDENT_CODE ON)
  set(BUILD_SHARED_LIBS ON)

  if (MSVC)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
    set(CMAKE_PDB_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
  endif ()
endif ()

# Version metadata
set(protobuf_VERSION_STRING "7.35.0")
set(protobuf_DESCRIPTION "Protocol Buffers")
set(protobuf_CONTACT "protobuf@googlegroups.com")

# Overrides for option dependencies
if (protobuf_BUILD_PROTOC_BINARIES OR protobuf_BUILD_TESTS)
  set(protobuf_BUILD_LIBPROTOC ON)
endif ()
if (NOT protobuf_BUILD_PROTOBUF_BINARIES)
  set(protobuf_INSTALL OFF)
endif()

set(protobuf_VERSION_REGEX "^([0-9]+)\\.([0-9]+)\\.([0-9]+)([-]rc[-]|\\.)?([0-9]*)$")
string(REGEX REPLACE     "${protobuf_VERSION_REGEX}" "\\1"
  protobuf_VERSION_MAJOR "${protobuf_VERSION_STRING}")
string(REGEX REPLACE     "${protobuf_VERSION_REGEX}" "\\2"
  protobuf_VERSION_MINOR "${protobuf_VERSION_STRING}")
string(REGEX REPLACE     "${protobuf_VERSION_REGEX}" "\\3"
  protobuf_VERSION_PATCH "${protobuf_VERSION_STRING}")
string(REGEX REPLACE     "${protobuf_VERSION_REGEX}" "\\5"
  protobuf_VERSION_PRERELEASE "${protobuf_VERSION_STRING}")

if (protobuf_FORCE_FETCH_DEPENDENCIES AND protobuf_LOCAL_DEPENDENCIES_ONLY)
  message(FATAL_ERROR "Conflicting options protobuf_FORCE_FETCH_DEPENDENCIES and protobuf_LOCAL_DEPENDENCIES_ONLY both set")
endif()

set(protobuf_VERSION
  "${protobuf_VERSION_MINOR}.${protobuf_VERSION_PATCH}")

if(protobuf_VERSION_PRERELEASE)
  message(STATUS "${protobuf_VERSION_PRERELEASE}")
  set(protobuf_VERSION "${protobuf_VERSION}.${protobuf_VERSION_PRERELEASE}")
else()
  set(protobuf_VERSION "${protobuf_VERSION}.0")
endif()
message(STATUS "protobuf version: ${protobuf_VERSION}")

find_package(Threads REQUIRED)

if (protobuf_BUILD_SHARED_LIBS)
  set(protobuf_SHARED_OR_STATIC "SHARED")
else (protobuf_BUILD_SHARED_LIBS)
  set(protobuf_SHARED_OR_STATIC "STATIC")
  set(ABSL_MSVC_STATIC_RUNTIME ${protobuf_MSVC_STATIC_RUNTIME})
  if (protobuf_MSVC_STATIC_RUNTIME)
      set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded$<$<CONFIG:Debug>:Debug>)
  else()
      set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded$<$<CONFIG:Debug>:Debug>DLL)
  endif()
endif (protobuf_BUILD_SHARED_LIBS)

SET(CMAKE_WINDOWS_EXPORT_ALL_SYMBOLS ON)

include_directories(
  ${protobuf_BINARY_DIR}
  ${protobuf_SOURCE_DIR}
  ${protobuf_BINARY_DIR}/src
  ${protobuf_SOURCE_DIR}/src)

if (protobuf_BUILD_TESTS)
  include(${protobuf_SOURCE_DIR}/cmake/gtest.cmake)
endif (protobuf_BUILD_TESTS)

include(${protobuf_SOURCE_DIR}/cmake/abseil-cpp.cmake)

if (protobuf_BUILD_PROTOBUF_BINARIES)
  include(${protobuf_SOURCE_DIR}/cmake/utf8_range.cmake)
  include(${protobuf_SOURCE_DIR}/cmake/libprotobuf-lite.cmake)
  if (NOT DEFINED protobuf_LIB_PROTOBUF_LITE)
    set(protobuf_LIB_PROTOBUF_LITE libprotobuf-lite)
  endif ()
  include(${protobuf_SOURCE_DIR}/cmake/libprotobuf.cmake)
  if (NOT DEFINED protobuf_LIB_PROTOBUF)
    set(protobuf_LIB_PROTOBUF libprotobuf)
  endif ()
  if (protobuf_BUILD_LIBPROTOC)
    include(${protobuf_SOURCE_DIR}/cmake/libprotoc.cmake)
    if (NOT DEFINED protobuf_LIB_PROTOC)
      set(protobuf_LIB_PROTOC libprotoc)
    endif ()
  endif ()
  if (protobuf_BUILD_LIBUPB)
    include(${protobuf_SOURCE_DIR}/cmake/libupb.cmake)
    if (NOT DEFINED protobuf_LIB_UPB)
      set(protobuf_LIB_UPB libupb)
    endif ()
    include(${protobuf_SOURCE_DIR}/cmake/upb_generators.cmake)
  endif ()
  if (protobuf_BUILD_PROTOC_BINARIES)
    include(${protobuf_SOURCE_DIR}/cmake/protoc.cmake)
    if (NOT DEFINED protobuf_PROTOC_EXE)
      set(protobuf_PROTOC_EXE protoc)
    endif ()
  endif ()
endif ()

if (protobuf_BUILD_TESTS)
  enable_testing()
  include(${protobuf_SOURCE_DIR}/cmake/tests.cmake)
endif (protobuf_BUILD_TESTS)

if (protobuf_BUILD_CONFORMANCE)
  include(${protobuf_SOURCE_DIR}/cmake/conformance.cmake)
endif (protobuf_BUILD_CONFORMANCE)

if (protobuf_INSTALL)
  include(${protobuf_SOURCE_DIR}/cmake/install.cmake)
endif (protobuf_INSTALL)

if (protobuf_BUILD_EXAMPLES)
  include(${protobuf_SOURCE_DIR}/cmake/examples.cmake)
endif (protobuf_BUILD_EXAMPLES)

if(protobuf_VERBOSE)
  message(STATUS "Protocol Buffers Configuring done")
endif(protobuf_VERBOSE)
