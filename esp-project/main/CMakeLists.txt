# Collect runtime C sources
file(GLOB RUNTIME_SOURCES
    "${CMAKE_CURRENT_SOURCE_DIR}/../../runtime/src/*.c"
)

idf_component_register(
    SRCS ${RUNTIME_SOURCES}
    INCLUDE_DIRS "${CMAKE_CURRENT_SOURCE_DIR}/../../runtime/include"
    PRIV_REQUIRES bt driver esp_wifi esp_http_client mqtt nvs_flash esp_timer esp_adc esp_driver_ledc esp_driver_gpio esp_driver_i2c esp_driver_spi esp_driver_uart esp_driver_touch_sens esp_driver_tsens app_update esp_https_ota lwip json esp_netif esp_event
    WHOLE_ARCHIVE
)

# Link the compiled BASIC program object file
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/basic_program.o")
    target_link_libraries(${COMPONENT_LIB} INTERFACE
        "${CMAKE_CURRENT_SOURCE_DIR}/basic_program.o"
    )
endif()
