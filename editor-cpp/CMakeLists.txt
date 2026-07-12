cmake_minimum_required(VERSION 3.24)

project(MiniForgeQtEditor VERSION 0.9.3.4 LANGUAGES CXX)

include(GNUInstallDirs)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

option(MINIFORGE_EDITOR_BUILD_APP "Build the MiniForge Qt editor application" ON)
option(MINIFORGE_EDITOR_BUILD_TESTS "Build MiniForge Qt editor CTest smoke tests" ON)
option(MINIFORGE_EDITOR_ENABLE_QML_LINT "Add a CMake/CTest target for qmllint when available" ON)
set(MINIFORGE_EDITOR_RUST_PROFILE "debug" CACHE STRING "Cargo profile for the Rust editor bridge: debug or release")
set_property(CACHE MINIFORGE_EDITOR_RUST_PROFILE PROPERTY STRINGS debug release)

if(NOT MINIFORGE_EDITOR_RUST_PROFILE STREQUAL "debug" AND NOT MINIFORGE_EDITOR_RUST_PROFILE STREQUAL "release")
    message(FATAL_ERROR "MINIFORGE_EDITOR_RUST_PROFILE must be 'debug' or 'release'")
endif()

get_filename_component(MINIFORGE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/.." ABSOLUTE)
set(MINIFORGE_QML_ROOT "${MINIFORGE_ROOT}/editor-qml")
set(MINIFORGE_RUST_TARGET_DIR "${CMAKE_BINARY_DIR}/rust")
set(MINIFORGE_DEFAULT_PROJECT "${MINIFORGE_ROOT}/projects/DefaultProject" CACHE PATH "Project used by editor smoke tests")
set(MINIFORGE_RUST_PROFILE_DIR "${MINIFORGE_EDITOR_RUST_PROFILE}")
set(MINIFORGE_RUST_LIBRARY
    "${MINIFORGE_RUST_TARGET_DIR}/${MINIFORGE_RUST_PROFILE_DIR}/${CMAKE_SHARED_LIBRARY_PREFIX}miniforge${CMAKE_SHARED_LIBRARY_SUFFIX}"
)

find_program(CARGO_EXECUTABLE cargo REQUIRED)
find_package(Qt6 6.5 REQUIRED COMPONENTS Widgets Quick QuickWidgets Qml)
enable_testing()

set(MINIFORGE_CARGO_PROFILE_ARGS)
if(MINIFORGE_EDITOR_RUST_PROFILE STREQUAL "release")
    list(APPEND MINIFORGE_CARGO_PROFILE_ARGS --release)
endif()

add_custom_target(miniforge_rust_editor_bridge
    COMMAND "${CARGO_EXECUTABLE}" rustc --features editor_ffi --lib --crate-type cdylib --target-dir "${MINIFORGE_RUST_TARGET_DIR}" ${MINIFORGE_CARGO_PROFILE_ARGS}
    WORKING_DIRECTORY "${MINIFORGE_ROOT}"
    COMMENT "Building Rust EditorCore C ABI bridge"
    VERBATIM
)

add_library(miniforge_rust SHARED IMPORTED GLOBAL)
set_target_properties(miniforge_rust PROPERTIES IMPORTED_LOCATION "${MINIFORGE_RUST_LIBRARY}")

add_library(miniforge_editor_bridge_headers INTERFACE)
target_include_directories(miniforge_editor_bridge_headers INTERFACE
    "${MINIFORGE_ROOT}/include"
)

if(APPLE)
    set(MINIFORGE_INSTALL_RPATH "@loader_path/../${CMAKE_INSTALL_LIBDIR}")
elseif(UNIX)
    set(MINIFORGE_INSTALL_RPATH "$ORIGIN/../${CMAKE_INSTALL_LIBDIR}")
else()
    set(MINIFORGE_INSTALL_RPATH "")
endif()

if(MINIFORGE_EDITOR_BUILD_APP)
add_executable(miniforge_qt_editor
    src/main.cpp
    src/MfBridge.cpp
    src/MfBridge.h
    src/MfModels.cpp
    src/MfModels.h
    src/ViewportWidget.cpp
    src/ViewportWidget.h
    src/SpriteEditorWidget.cpp
    src/SpriteEditorWidget.h
    src/MainWindow.cpp
    src/MainWindow.h
)

add_dependencies(miniforge_qt_editor miniforge_rust_editor_bridge)

target_compile_definitions(miniforge_qt_editor PRIVATE
    MF_QML_ROOT="${MINIFORGE_QML_ROOT}"
    MF_QML_INSTALL_SUBDIR="${CMAKE_INSTALL_DATADIR}/miniforge/editor-qml"
    MF_DEFAULT_PROJECT="${MINIFORGE_DEFAULT_PROJECT}"
)

target_link_libraries(miniforge_qt_editor PRIVATE
    Qt6::Widgets
    Qt6::Quick
    Qt6::QuickWidgets
    Qt6::Qml
    miniforge_rust
    miniforge_editor_bridge_headers
)

install(TARGETS miniforge_qt_editor
    BUNDLE DESTINATION .
    RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
)

install(FILES "${MINIFORGE_RUST_LIBRARY}"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}"
)

install(FILES "${MINIFORGE_ROOT}/include/miniforge_editor_bridge.h"
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/miniforge"
)

if(APPLE OR UNIX)
    set_target_properties(miniforge_qt_editor PROPERTIES
        BUILD_RPATH "${MINIFORGE_RUST_TARGET_DIR}/${MINIFORGE_RUST_PROFILE_DIR}"
        INSTALL_RPATH "${MINIFORGE_INSTALL_RPATH}"
    )
endif()
endif()

if(MINIFORGE_EDITOR_BUILD_TESTS)
add_executable(miniforge_editor_bridge_smoke_test
    tests/bridge_smoke.cpp
)

add_dependencies(miniforge_editor_bridge_smoke_test miniforge_rust_editor_bridge)

target_link_libraries(miniforge_editor_bridge_smoke_test PRIVATE
    miniforge_rust
    miniforge_editor_bridge_headers
)

add_test(
    NAME miniforge_editor_bridge_smoke
    COMMAND miniforge_editor_bridge_smoke_test "${MINIFORGE_DEFAULT_PROJECT}"
)

set_tests_properties(miniforge_editor_bridge_smoke PROPERTIES
    LABELS "editor;ffi;smoke"
)

if(APPLE OR UNIX)
    set_target_properties(miniforge_editor_bridge_smoke_test PROPERTIES
        BUILD_RPATH "${MINIFORGE_RUST_TARGET_DIR}/${MINIFORGE_RUST_PROFILE_DIR}"
    )
endif()
endif()

install(DIRECTORY "${MINIFORGE_QML_ROOT}/"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/miniforge/editor-qml"
)

if(MINIFORGE_EDITOR_ENABLE_QML_LINT)
    find_program(QMLLINT_EXECUTABLE qmllint)
    if(QMLLINT_EXECUTABLE)
        file(GLOB_RECURSE MINIFORGE_QML_FILES CONFIGURE_DEPENDS
            "${MINIFORGE_QML_ROOT}/*.qml"
        )
        add_custom_target(miniforge_editor_qml_lint
            COMMAND "${QMLLINT_EXECUTABLE}" ${MINIFORGE_QML_FILES}
            WORKING_DIRECTORY "${MINIFORGE_ROOT}"
            COMMENT "Linting MiniForge editor QML"
            VERBATIM
        )
        add_test(
            NAME miniforge_editor_qml_lint
            COMMAND "${QMLLINT_EXECUTABLE}" ${MINIFORGE_QML_FILES}
            WORKING_DIRECTORY "${MINIFORGE_ROOT}"
        )
        set_tests_properties(miniforge_editor_qml_lint PROPERTIES
            LABELS "editor;qml;lint"
        )
    else()
        message(STATUS "qmllint was not found; miniforge_editor_qml_lint target will not be generated")
    endif()
endif()

set(MINIFORGE_EDITOR_CHECK_TARGETS)
if(TARGET miniforge_qt_editor)
    list(APPEND MINIFORGE_EDITOR_CHECK_TARGETS miniforge_qt_editor)
endif()
if(TARGET miniforge_editor_bridge_smoke_test)
    list(APPEND MINIFORGE_EDITOR_CHECK_TARGETS miniforge_editor_bridge_smoke_test)
endif()
if(TARGET miniforge_editor_qml_lint)
    list(APPEND MINIFORGE_EDITOR_CHECK_TARGETS miniforge_editor_qml_lint)
endif()
if(MINIFORGE_EDITOR_CHECK_TARGETS)
    add_custom_target(miniforge_editor_checks
        DEPENDS ${MINIFORGE_EDITOR_CHECK_TARGETS}
    )
endif()

set(CPACK_PACKAGE_NAME "MiniForgeQtEditor")
set(CPACK_PACKAGE_VENDOR "MiniForge")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "MiniForge Qt editor shell")
set(CPACK_GENERATOR "TGZ;STGZ")
include(CPack)
