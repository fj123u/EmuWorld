#pragma once

#include <QString>
#include <QStringList>

struct EmulatorInfo {
    QString id;
    QString name;
    QString console;
    QString description;
    QString downloadUrl;
    QString executableName;
    QStringList supportedExtensions;
    QString website;
    QString archiveType; // "zip" ou "7z"
};

