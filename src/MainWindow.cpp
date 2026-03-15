#include "MainWindow.h"
#include "EmulatorManager.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QListWidget>
#include <QPushButton>
#include <QStackedWidget>
#include <QVBoxLayout>
#include <QWidget>

MainWindow::MainWindow(QWidget* parent)
    : QMainWindow(parent),
      m_emulatorManager(new EmulatorManager(this)),
      m_navList(nullptr),
      m_pages(nullptr),
      m_catalogPage(nullptr),
      m_libraryPage(nullptr),
      m_installedPage(nullptr),
      m_settingsPage(nullptr)
{
    setupUi();
    populateCatalog();
}

void MainWindow::setupUi()
{
    auto* central = new QWidget(this);
    auto* layout = new QHBoxLayout(central);

    m_navList = new QListWidget(central);
    m_navList->addItem("Catalog");
    m_navList->addItem("Library");
    m_navList->addItem("Installed");
    m_navList->addItem("Settings");
    m_navList->setCurrentRow(0);
    connect(m_navList, &QListWidget::currentRowChanged,
            this, &MainWindow::onNavChanged);

    m_pages = new QStackedWidget(central);

    m_catalogPage = new QWidget(m_pages);
    auto* catLayout = new QVBoxLayout(m_catalogPage);
    catLayout->addWidget(new QLabel("Emulator Catalog", m_catalogPage));

    m_libraryPage = new QWidget(m_pages);
    auto* libLayout = new QVBoxLayout(m_libraryPage);
    libLayout->addWidget(new QLabel("Game Library (à implémenter)", m_libraryPage));

    m_installedPage = new QWidget(m_pages);
    auto* instLayout = new QVBoxLayout(m_installedPage);
    instLayout->addWidget(new QLabel("Installed Emulators (à implémenter)", m_installedPage));

    m_settingsPage = new QWidget(m_pages);
    auto* setLayout = new QVBoxLayout(m_settingsPage);
    setLayout->addWidget(new QLabel("Settings (à implémenter)", m_settingsPage));

    m_pages->addWidget(m_catalogPage);
    m_pages->addWidget(m_libraryPage);
    m_pages->addWidget(m_installedPage);
    m_pages->addWidget(m_settingsPage);

    layout->addWidget(m_navList);
    layout->addWidget(m_pages, 1);

    setCentralWidget(central);
    setWindowTitle("Universal Emulator Hub (C++ / Qt)");
    resize(1280, 800);
}

void MainWindow::populateCatalog()
{
    auto* layout = qobject_cast<QVBoxLayout*>(m_catalogPage->layout());
    if (!layout) return;

    const auto& catalog = m_emulatorManager->catalog();
    for (const auto& emu : catalog) {
        auto* row = new QWidget(m_catalogPage);
        auto* rowLayout = new QHBoxLayout(row);
        rowLayout->addWidget(new QLabel(emu.name + " — " + emu.console, row));

        auto* installBtn = new QPushButton("Install", row);
        installBtn->setProperty("emuId", emu.id);
        connect(installBtn, &QPushButton::clicked,
                this, &MainWindow::onInstallClicked);
        rowLayout->addWidget(installBtn);

        auto* launchBtn = new QPushButton("Launch", row);
        launchBtn->setProperty("emuId", emu.id);
        connect(launchBtn, &QPushButton::clicked,
                this, &MainWindow::onLaunchClicked);
        rowLayout->addWidget(launchBtn);

        layout->addWidget(row);
    }

    layout->addStretch(1);
}

void MainWindow::onNavChanged(int row)
{
    m_pages->setCurrentIndex(row);
}

void MainWindow::onInstallClicked()
{
    auto* btn = qobject_cast<QPushButton*>(sender());
    if (!btn) return;
    const QString id = btn->property("emuId").toString();
    m_emulatorManager->installEmulator(id);
}

void MainWindow::onLaunchClicked()
{
    auto* btn = qobject_cast<QPushButton*>(sender());
    if (!btn) return;
    const QString id = btn->property("emuId").toString();
    m_emulatorManager->launchEmulator(id);
}

