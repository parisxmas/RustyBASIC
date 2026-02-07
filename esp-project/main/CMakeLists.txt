# Collect runtime C sources
file(GLOB RUNTIME_SOURCES
    "${CMAKE_CURRENT_SOURCE_DIR}/../../runtime/src/*.c"
)

idf_component_register(
    SRCS ${RUNTIME_SOURCES}
    INCLUDE_DIRS "${CMAKE_CURRENT_SOURCE_DIR}/../../runtime/include"
)

# Link the compiled BASIC program object file
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/basic_program.o")
    target_link_libraries(${COMPONENT_LIB} INTERFACE
        "${CMAKE_CURRENT_SOURCE_DIR}/basic_program.o"
    )
endif()
