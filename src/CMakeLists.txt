set(SOURCES
    main.cpp
    MainWindow.cpp
    EmulatorManager.cpp
)

set(HEADERS
    MainWindow.h
    EmulatorManager.h
    models/Emulator.h
    models/Rom.h
)

add_executable(UniversalEmulatorHubCpp
    ${SOURCES}
    ${HEADERS}
)

target_link_libraries(UniversalEmulatorHubCpp
    Qt6::Widgets
    Qt6::Network
)

