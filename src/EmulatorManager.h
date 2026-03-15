#pragma once

#include "models/Emulator.h"
#include "models/Rom.h"
#include <QObject>
#include <QVector>

class QNetworkAccessManager;
class QNetworkReply;

class EmulatorManager : public QObject {
    Q_OBJECT
public:
    explicit EmulatorManager(QObject* parent = nullptr);

    const QVector<EmulatorInfo>& catalog() const { return m_catalog; }

signals:
    void installProgress(const QString& emulatorId, int percent, const QString& status);
    void installFinished(const QString& emulatorId, bool success, const QString& message);

public slots:
    void installEmulator(const QString& id);
    void launchEmulator(const QString& id, const QString& romPath = QString());

private slots:
    void onDownloadFinished(QNetworkReply* reply);

private:
    void loadBuiltInCatalog();

    QVector<EmulatorInfo> m_catalog;
    QNetworkAccessManager* m_net;
};

