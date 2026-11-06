"use strict";

/**
 * Formatiert einen numerischen Wert nach deutschem Standard (de-DE).
 * Gibt "--" zurück, falls der Wert ungültig, null oder undefined ist.
 * @param {number|null|undefined} val - Der zu formatierende Wert
 * @param {number} decimals - Anzahl der Nachkommastellen (Standard: 1)
 */
function formatDE(val, decimals = 1) {
    if (
        val === null ||
        val === undefined ||
        !Number.isFinite(Number(val))
    ) {
        return "--";
    }

    return Number(val).toLocaleString("de-DE", {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals
    });
}

// Globale Variablen für die Chart.js-Instanzen (bestehend + neu)
let cTemp;
let cHum;
let cPress;
let cCO2;
let cNH3;
let cCO;
let cN2;

// Zustandsvariablen für die Datenabfrage und Initialisierung
let lastFingerprint = null;
let historyRequestRunning = false;
let currentRequestRunning = false;
let dashboardInitialized = false;

// Polling-Intervall in Millisekunden (hier alle 2 Sekunden)
const POLL_INTERVAL = 2000;

/**
 * Erstellt ein neues Linien-Diagramm mit Chart.js.
 * @param {string} canvasId - Die ID des Canvas-Elements im HTML
 * @param {string} label - Die Beschriftung des Datensatzes
 * @param {string} color - Die HEX-Farbe für Linie und Hintergrund
 */
function createChart(canvasId, label, color) {
    const canvas = document.getElementById(canvasId);

    if (!canvas || typeof Chart === "undefined") {
        console.error("Chart.js oder Canvas fehlt:", canvasId);
        return null;
    }

    return new Chart(canvas.getContext("2d"), {
        type: "line",
        data: {
            labels: [],
            datasets: [{
                label: label,
                data: [],
                borderColor: color,
                backgroundColor: color + "22", // Leichte Transparenz für die Füllung
                borderWidth: 2,
                fill: true,
                tension: 0.2,
                spanGaps: false
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false, // Animationen für Performance ausschalten
            plugins: {
                legend: {
                    labels: {
                        color: "#e0e0e0"
                    }
                }
            },
            scales: {
                x: {
                    ticks: { color: "#a0a0a0" },
                    grid: { color: "#2d2d2d" }
                },
                y: {
                    ticks: { color: "#a0a0a0" },
                    grid: { color: "#2d2d2d" }
                }
            }
        }
    });
}

/**
 * Aktualisiert den optischen Status (Farbklasse) einer Dashboard-Kachel und den Statustext.
 * @param {string} elementId - ID des Kachel-Elements
 * @param {string} statusId - ID des Text-Elements für den Status
 * @param {string} state - Statusklasse ("green", "orange", "red")
 * @param {string} text - Anzuzeigender Status-Text
 */
function setKachelStatus(elementId, statusId, state, text) {
    const kachel = document.getElementById(elementId);
    const status = document.getElementById(statusId);

    if (kachel) {
        kachel.classList.remove(
            "status-green",
            "status-orange",
            "status-red"
        );

        kachel.classList.add("status-" + state);
    }

    if (status) {
        status.textContent = text;
    }
}

/**
 * Ermittelt anhand von Wertebereichen den passenden Status für einen Messwert.
 * @param {number|null} value - Der Messwert
 * @param {string} elementId - ID der Kachel
 * @param {string} statusId - ID des Status-Textes
 * @param {Object} ranges - Objekt mit Prüf-Funktionen (green, orange) und Texten
 */
function setStatusForValue(value, elementId, statusId, ranges) {
    if (
        value === null ||
        value === undefined ||
        !Number.isFinite(Number(value))
    ) {
        setKachelStatus(
            elementId,
            statusId,
            "orange",
            "Keine Messdaten"
        );
        return;
    }

    const number = Number(value);

    if (ranges.green(number)) {
        setKachelStatus(
            elementId,
            statusId,
            "green",
            ranges.greenText
        );
    } else if (ranges.orange(number)) {
        setKachelStatus(
            elementId,
            statusId,
            "orange",
            ranges.orangeText
        );
    } else {
        setKachelStatus(
            elementId,
            statusId,
            "red",
            ranges.redText
        );
    }
}

/**
 * Aktualisiert den globalen Verbindungsstatus in der UI.
 * @param {string} text - Statustext
 * @param {string} color - Textfarbe (optional)
 */
function setConnectionStatus(text, color) {
    const element = document.getElementById("connection-status");

    if (!element) return;

    element.textContent = text;

    if (color) {
        element.style.color = color;
    }
}

/**
 * Setzt das gesamte Dashboard auf Standard-/Leermerte zurück.
 */
function clearDashboard() {
    // Aktuelle Messwerte auf Platzhalter setzen
    ["val-temp", "val-hum", "val-press", "val-co2", "val-nh3", "val-co", "val-n2"].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = "--";
    });

    // Status-Texte zurücksetzen
    [
        "st-temp",
        "st-hum",
        "st-press",
        "st-co2",
        "st-nh3",
        "st-co",
        "st-n2"
    ].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = "Keine Messdaten";
    });

    // Kachel-Farben zurücksetzen
    [
        ["kachel-temp", "st-temp"],
        ["kachel-hum", "st-hum"],
        ["kachel-press", "st-press"],
        ["kachel-co2", "st-co2"],
        ["kachel-nh3", "st-nh3"],
        ["kachel-co", "st-co"],
        ["kachel-n2", "st-n2"]
    ].forEach(([kachel, status]) => {
        setKachelStatus(kachel, status, "orange", "Keine Messdaten");
    });

    // Statistik-Werte zurücksetzen
    [
        "st-t-min",
        "st-t-max",
        "st-t-avg",
        "st-c-min",
        "st-c-max",
        "st-c-avg"
    ].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = "--";
    });

    // Alle Charts leeren
    [cTemp, cHum, cPress, cCO2, cNH3, cCO, cN2].forEach(chart => {
        if (chart) {
            chart.data.labels = [];
            chart.data.datasets[0].data = [];
            chart.update("none");
        }
    });
}

/**
 * Aktualisiert die Anzeige der aktuellen Messwerte und deren Bewertungsstatus.
 * @param {Object} data - Objektdaten von der API (/api/current)
 */
function updateCurrentValues(data) {
    document.getElementById("val-temp").textContent = formatDE(data.temperature);
    document.getElementById("val-hum").textContent = formatDE(data.humidity);
    document.getElementById("val-press").textContent = formatDE(data.pressure);
    document.getElementById("val-co2").textContent = formatDE(data.co2, 0);
    
    // Neue Gaswerte setzen (Beispiel: NH3 & CO mit 1 Nachkommastelle, N2 z.B. 1 oder 2)
    const elNh3 = document.getElementById("val-nh3");
    if (elNh3) elNh3.textContent = formatDE(data.nh3, 1);

    const elCo = document.getElementById("val-co");
    if (elCo) elCo.textContent = formatDE(data.co, 1);

    const elN2 = document.getElementById("val-n2");
    if (elN2) elN2.textContent = formatDE(data.n2, 1);

    // Bewertung der Temperatur (Optimal: 19 - 24 °C)
    setStatusForValue(data.temperature, "kachel-temp", "st-temp", {
        green: v => v >= 19 && v <= 24,
        orange: v => (v >= 16 && v < 19) || (v > 24 && v <= 27),
        greenText: "Optimal",
        orangeText: "Akzeptabel",
        redText: "Zu kalt / Zu warm"
    });

    // Bewertung der Luftfeuchtigkeit (Optimal: 40 - 60 % rF)
    setStatusForValue(data.humidity, "kachel-hum", "st-hum", {
        green: v => v >= 40 && v <= 60,
        orange: v => (v >= 30 && v < 40) || (v > 60 && v <= 70),
        greenText: "Optimal",
        orangeText: "Trocken / Feucht",
        redText: "Zu trocken / Zu feucht"
    });

    // Bewertung des Luftdrucks (Normaldruck: 1000 - 1025 hPa)
    setStatusForValue(data.pressure, "kachel-press", "st-press", {
        green: v => v >= 1000 && v <= 1025,
        orange: v => v < 1000 || v > 1025,
        greenText: "Normaldruck",
        orangeText: "Abweichender Luftdruck",
        redText: "Abweichender Luftdruck"
    });

    // Bewertung des CO2-Gehalts (Gut: < 1000 ppm)
    setStatusForValue(data.co2, "kachel-co2", "st-co2", {
        green: v => v < 1000,
        orange: v => v >= 1000 && v <= 2000,
        greenText: "Gute Luft",
        orangeText: "Lüften empfohlen",
        redText: "Dringend lüften!"
    });

    // Bewertung von NH3 (Ammoniak) - Beispielgrenzwerte ( ppm / mg/m³ je nach Sensor)
    setStatusForValue(data.nh3, "kachel-nh3", "st-nh3", {
        green: v => v < 25,
        orange: v => v >= 25 && v < 50,
        greenText: "Sicher",
        orangeText: "Erhöht",
        redText: "Gefährliche Konzentration!"
    });

    // Bewertung von CO (Kohlenmonoxid) - Beispielgrenzwerte
    setStatusForValue(data.co, "kachel-co", "st-co", {
        green: v => v < 9,
        orange: v => v >= 9 && v < 35,
        greenText: "Normal",
        orangeText: "Erhöht",
        redText: "Achtung: Giftiges Gas!"
    });

    // Bewertung von N2 (Stickstoff) - Beispiel (z. B. Umgebungsluft ~78% bzw. 780000 ppm)
    setStatusForValue(data.n2, "kachel-n2", "st-n2", {
        green: v => v >= 75 && v <= 81, // Angenommen in Prozent (%)
        orange: v => (v >= 70 && v < 75) || (v > 81 && v <= 85),
        greenText: "Normaler Wert",
        orangeText: "Abweichend",
        redText: "Kritischer Wert"
    });
}

/**
 * Pollt regelmäßig die Schnittstelle für aktuelle Messwerte (`/api/current`)
 * und erkennt Änderungen anhand eines Daten-Fingerprints.
 */
async function checkForNewData() {
    if (currentRequestRunning) return;

    currentRequestRunning = true;

    try {
        const response = await fetch("/api/current", {
            cache: "no-store"
        });

        if (!response.ok) {
            throw new Error("HTTP " + response.status);
        }

        const data = await response.json();

        if (data.error) {
            throw new Error(data.error);
        }

        // Verbindung erfolgreich hergestellt
        setConnectionStatus("Verbunden", "#51cf66");

        // Erstelle einen Fingerprint inkl. der neuen Gase
        const fingerprint = JSON.stringify([
            data.timestamp,
            data.temperature,
            data.humidity,
            data.pressure,
            data.co2,
            data.gas_raw,
            data.nh3,
            data.co,
            data.n2
        ]);

        const isNewData = lastFingerprint !== null &&
            fingerprint !== lastFingerprint;

        lastFingerprint = fingerprint;

        // UI mit aktuellen Werten füttern
        updateCurrentValues(data);

        // Dashboard initialisieren oder bei neuen Daten aktualisieren
        if (!dashboardInitialized) {
            dashboardInitialized = true;
            await updateDashboard();
        } else if (isNewData) {
            console.info("Neue Messwerte erkannt:", data.timestamp);
            await updateDashboard();
        }

    } catch (error) {
        console.error("Fehler bei der Datenbankprüfung:", error);
        setConnectionStatus("Verbindung fehlgeschlagen", "#ff6b6b");
    } finally {
        currentRequestRunning = false;
    }
}

/**
 * Lädt die Verlaufs- und Statistikdaten für das gewählte Zeitfenster (`/api/history`)
 * und aktualisiert die Charts sowie Textstatistiken.
 */
async function updateDashboard() {
    if (historyRequestRunning) {
        return;
    }

    historyRequestRunning = true;

    const timeframe = document.getElementById("timeframe");
    const hours = timeframe ? timeframe.value : "24";

    // Text für das Statistik-Label je nach gewähltem Zeitfenster anpassen
    const statsLabel = document.getElementById("stats-label");
    if (statsLabel) {
        statsLabel.textContent =
            hours === "1"
                ? "1 Stunde"
                : hours === "24"
                    ? "24h"
                    : "7 Tage";
    }

    try {
        const response = await fetch(
            `/api/history?hours=${encodeURIComponent(hours)}`,
            { cache: "no-store" }
        );

        if (!response.ok) {
            throw new Error("API-Fehler: HTTP " + response.status);
        }

        const data = await response.json();

        if (data.error) {
            throw new Error(data.error);
        }

        // Prüfen, ob Timestamps vorhanden sind
        if (
            !Array.isArray(data.timestamps) ||
            data.timestamps.length === 0
        ) {
            clearDashboard();

            const lastUpdateEl = document.getElementById("last-update");
            if (lastUpdateEl) {
                lastUpdateEl.textContent = "Keine Messdaten im gewählten Zeitraum";
            }

            return;
        }

        // Letzten Zeitstempel anzeigen
        const last = data.timestamps.length - 1;
        const lastUpdateEl = document.getElementById("last-update");
        if (lastUpdateEl) {
            lastUpdateEl.textContent = data.timestamps[last];
        }

        // Statistiken (Min, Max, Durchschnitt) aktualisieren, falls vorhanden
        if (data.stats) {
            const setStatText = (id, val, dec = 1) => {
                const el = document.getElementById(id);
                if (el) el.textContent = formatDE(val, dec);
            };

            setStatText("st-t-min", data.stats.temp_min);
            setStatText("st-t-max", data.stats.temp_max);
            setStatText("st-t-avg", data.stats.temp_avg);

            setStatText("st-c-min", data.stats.co2_min, 0);
            setStatText("st-c-max", data.stats.co2_max, 0);
            setStatText("st-c-avg", data.stats.co2_avg, 0);
        }

        // Charts mit den neuen Verlaufsdaten versorgen (inkl. NH3, CO, N2)
        const charts = [
            [cTemp, data.temperature],
            [cHum, data.humidity],
            [cPress, data.pressure],
            [cCO2, data.co2],
            [cNH3, data.nh3],
            [cCO, data.co],
            [cN2, data.n2]
        ];

        charts.forEach(([chart, values]) => {
            if (!chart || !values) return;

            chart.data.labels = data.timestamps;
            chart.data.datasets[0].data = values;
            chart.update("none");
        });

    } catch (error) {
        console.error("Fehler beim Laden der Verlaufsdaten:", error);
        setConnectionStatus("Fehler beim Laden", "#ff6b6b");
    } finally {
        historyRequestRunning = false;
    }
}

/**
 * Startet den CSV-Export für das aktuell ausgewählte Zeitfenster.
 */
function downloadCSV() {
    const timeframe = document.getElementById("timeframe");
    const hours = timeframe ? timeframe.value : "24";

    window.location.href =
        `/api/export/csv?hours=${encodeURIComponent(hours)}`;
}

// Event-Listener beim Laden des DOMs (Initialisierung der App)
document.addEventListener("DOMContentLoaded", () => {
    // Bestehende Charts initialisieren
    cTemp = createChart("chartTemp", "Temperatur (°C)", "#ff6b6b");
    cHum = createChart("chartHum", "Luftfeuchte (% rF)", "#4dabf7");
    cPress = createChart("chartPress", "Luftdruck (hPa)", "#cc5de8");
    cCO2 = createChart("chartCO2", "CO₂ (ppm)", "#51cf66");

    // Neue Charts initialisieren (Farben frei anpassbar)
    cNH3 = createChart("chartNH3", "NH₃ (ppm)", "#fab005");
    cCO = createChart("chartCO", "CO (ppm)", "#ff922b");
    cN2 = createChart("chartN2", "N₂ (%)", "#20c997");

    // Event-Listener für Änderungen am Zeitfenster-Dropdown
    const timeframeEl = document.getElementById("timeframe");
    if (timeframeEl) {
        timeframeEl.addEventListener("change", updateDashboard);
    }

    // Event-Listener für den CSV-Export-Button
    const exportBtn = document.getElementById("export-button");
    if (exportBtn) {
        exportBtn.addEventListener("click", downloadCSV);
    }

    // Erster Datenabruf beim Start
    checkForNewData();

    // Regelmäßiges Polling starten
    setInterval(checkForNewData, POLL_INTERVAL);
});