#pragma once

#include <QMainWindow>

class QListWidget;
class QStackedWidget;
class EmulatorManager;

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget* parent = nullptr);

private slots:
    void onNavChanged(int row);
    void onInstallClicked();
    void onLaunchClicked();

private:
    void setupUi();
    void populateCatalog();

    EmulatorManager* m_emulatorManager;
    QListWidget* m_navList;
    QStackedWidget* m_pages;

    QWidget* m_catalogPage;
    QWidget* m_libraryPage;
    QWidget* m_installedPage;
    QWidget* m_settingsPage;
};

