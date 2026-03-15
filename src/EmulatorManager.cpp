#include "EmulatorManager.h"

#include <QDesktopServices>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>

EmulatorManager::EmulatorManager(QObject* parent)
    : QObject(parent),
      m_net(new QNetworkAccessManager(this))
{
    loadBuiltInCatalog();
    connect(m_net, &QNetworkAccessManager::finished,
            this, &EmulatorManager::onDownloadFinished);
}

void EmulatorManager::loadBuiltInCatalog()
{
    EmulatorInfo mgba;
    mgba.id = "mgba";
    mgba.name = "mGBA";
    mgba.console = "Game Boy Advance";
    mgba.description = "Fast and accurate GBA emulator.";
    mgba.downloadUrl = "https://example.com/mgba.zip";
    mgba.executableName = "mGBA.exe";
    mgba.supportedExtensions = {"gba", "gb", "gbc"};
    mgba.website = "https://mgba.io";
    mgba.archiveType = "zip";

    m_catalog.append(mgba);
}

void EmulatorManager::installEmulator(const QString& id)
{
    auto it = std::find_if(m_catalog.begin(), m_catalog.end(),
                           [&](const EmulatorInfo& e){ return e.id == id; });
    if (it == m_catalog.end()) {
        emit installFinished(id, false, "Emulator not found");
        return;
    }

    QUrl url(it->downloadUrl);
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::UserAgentHeader,
                  "UniversalEmulatorHubCpp/1.0 (Qt; Windows)");

    emit installProgress(id, 10, "Downloading...");
    auto* reply = m_net->get(req);
    reply->setProperty("emulatorId", id);
}

void EmulatorManager::onDownloadFinished(QNetworkReply* reply)
{
    const QString id = reply->property("emulatorId").toString();

    if (reply->error() != QNetworkReply::NoError) {
        emit installFinished(id, false, QStringLiteral("Download failed: %1")
                                           .arg(reply->errorString()));
        reply->deleteLater();
        return;
    }

    // TODO: sauvegarder l'archive et l'extraire.
    emit installProgress(id, 100, "Downloaded (not extracted yet)");
    emit installFinished(id, true, "Archive downloaded (extraction not implemented)");
    reply->deleteLater();
}

void EmulatorManager::launchEmulator(const QString& id, const QString& /*romPath*/)
{
    auto it = std::find_if(m_catalog.begin(), m_catalog.end(),
                           [&](const EmulatorInfo& e){ return e.id == id; });
    if (it == m_catalog.end()) {
        emit installFinished(id, false, "Emulator not found");
        return;
    }

    // Pour l'instant, on ouvre simplement le site officiel dans le navigateur.
    QDesktopServices::openUrl(QUrl(it->website));
}

