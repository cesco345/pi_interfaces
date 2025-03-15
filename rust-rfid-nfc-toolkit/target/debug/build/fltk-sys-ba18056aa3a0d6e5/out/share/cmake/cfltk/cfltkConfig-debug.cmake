#----------------------------------------------------------------
# Generated CMake target import file for configuration "Debug".
#----------------------------------------------------------------

# Commands may need to know the format version.
set(CMAKE_IMPORT_FILE_VERSION 1)

# Import target "cfltk::cfltk" for configuration "Debug"
set_property(TARGET cfltk::cfltk APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(cfltk::cfltk PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "CXX"
  IMPORTED_LOCATION_DEBUG "/home/pi/rust/pi_afr/rust-rfid-nfc-toolkit/target/debug/build/fltk-sys-ba18056aa3a0d6e5/out/lib/libcfltk.a"
  )

list(APPEND _IMPORT_CHECK_TARGETS cfltk::cfltk )
list(APPEND _IMPORT_CHECK_FILES_FOR_cfltk::cfltk "/home/pi/rust/pi_afr/rust-rfid-nfc-toolkit/target/debug/build/fltk-sys-ba18056aa3a0d6e5/out/lib/libcfltk.a" )

# Commands beyond this point should not need to know the version.
set(CMAKE_IMPORT_FILE_VERSION)
