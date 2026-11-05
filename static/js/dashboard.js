"use strict";

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

let cTemp;
let cHum;
let cPress;
let cCO2;

let lastFingerprint = null;
let historyRequestRunning = false;
let currentRequestRunning = false;
let dashboardInitialized = false;

const POLL_INTERVAL = 2000;

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
                backgroundColor: color + "22",
                borderWidth: 2,
                fill: true,
                tension: 0.2,
                spanGaps: false
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            plugins: {
                legend: {
                    labels: {
                        color: "#e0e0e0"
                    }
                }
            },
            scales: {
                x: {
                    ticks: {
                        color: "#a0a0a0"
                    },
                    grid: {
                        color: "#2d2d2d"
                    }
                },
                y: {
                    ticks: {
                        color: "#a0a0a0"
                    },
                    grid: {
                        color: "#2d2d2d"
                    }
                }
            }
        }
    });
}

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

function setConnectionStatus(text, color) {
    const element = document.getElementById("connection-status");

    if (!element) return;

    element.textContent = text;

    if (color) {
        element.style.color = color;
    }
}

function clearDashboard() {
    ["val-temp", "val-hum", "val-press", "val-co2"].forEach(id => {
        document.getElementById(id).textContent = "--";
    });

    [
        "st-temp",
        "st-hum",
        "st-press",
        "st-co2"
    ].forEach(id => {
        document.getElementById(id).textContent = "Keine Messdaten";
    });

    [
        ["kachel-temp", "st-temp"],
        ["kachel-hum", "st-hum"],
        ["kachel-press", "st-press"],
        ["kachel-co2", "st-co2"]
    ].forEach(([kachel, status]) => {
        setKachelStatus(kachel, status, "orange", "Keine Messdaten");
    });

    [
        "st-t-min",
        "st-t-max",
        "st-t-avg",
        "st-c-min",
        "st-c-max",
        "st-c-avg"
    ].forEach(id => {
        document.getElementById(id).textContent = "--";
    });

    [cTemp, cHum, cPress, cCO2].forEach(chart => {
        if (chart) {
            chart.data.labels = [];
            chart.data.datasets[0].data = [];
            chart.update("none");
        }
    });
}

function updateCurrentValues(data) {
    document.getElementById("val-temp").textContent =
        formatDE(data.temperature);

    document.getElementById("val-hum").textContent =
        formatDE(data.humidity);

    document.getElementById("val-press").textContent =
        formatDE(data.pressure);

    document.getElementById("val-co2").textContent =
        formatDE(data.co2, 0);

    setStatusForValue(data.temperature, "kachel-temp", "st-temp", {
        green: v => v >= 19 && v <= 24,
        orange: v =>
            (v >= 16 && v < 19) ||
            (v > 24 && v <= 27),
        greenText: "Optimal",
        orangeText: "Akzeptabel",
        redText: "Zu kalt / Zu warm"
    });

    setStatusForValue(data.humidity, "kachel-hum", "st-hum", {
        green: v => v >= 40 && v <= 60,
        orange: v =>
            (v >= 30 && v < 40) ||
            (v > 60 && v <= 70),
        greenText: "Optimal",
        orangeText: "Trocken / Feucht",
        redText: "Zu trocken / Zu feucht"
    });

    setStatusForValue(data.pressure, "kachel-press", "st-press", {
        green: v => v >= 1000 && v <= 1025,
        orange: v => v < 1000 || v > 1025,
        greenText: "Normaldruck",
        orangeText: "Abweichender Luftdruck",
        redText: "Abweichender Luftdruck"
    });

    setStatusForValue(data.co2, "kachel-co2", "st-co2", {
        green: v => v < 1000,
        orange: v => v >= 1000 && v <= 2000,
        greenText: "Gute Luft",
        orangeText: "Lüften empfohlen",
        redText: "Dringend lüften!"
    });
}

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

        setConnectionStatus("Verbunden", "#51cf66");

        const fingerprint = JSON.stringify([
            data.timestamp,
            data.temperature,
            data.humidity,
            data.pressure,
            data.co2,
            data.gas_raw
        ]);

        const isNewData = lastFingerprint !== null &&
            fingerprint !== lastFingerprint;

        lastFingerprint = fingerprint;

        updateCurrentValues(data);

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

async function updateDashboard() {
    if (historyRequestRunning) {
        return;
    }

    historyRequestRunning = true;

    const timeframe = document.getElementById("timeframe");
    const hours = timeframe ? timeframe.value : "24";

    document.getElementById("stats-label").textContent =
        hours === "1"
            ? "1 Stunde"
            : hours === "24"
                ? "24h"
                : "7 Tage";

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

        if (
            !Array.isArray(data.timestamps) ||
            data.timestamps.length === 0
        ) {
            clearDashboard();

            document.getElementById("last-update").textContent =
                "Keine Messdaten im gewählten Zeitraum";

            return;
        }

        const last = data.timestamps.length - 1;

        document.getElementById("last-update").textContent =
            data.timestamps[last];

        if (data.stats) {
            document.getElementById("st-t-min").textContent =
                formatDE(data.stats.temp_min);

            document.getElementById("st-t-max").textContent =
                formatDE(data.stats.temp_max);

            document.getElementById("st-t-avg").textContent =
                formatDE(data.stats.temp_avg);

            document.getElementById("st-c-min").textContent =
                formatDE(data.stats.co2_min, 0);

            document.getElementById("st-c-max").textContent =
                formatDE(data.stats.co2_max, 0);

            document.getElementById("st-c-avg").textContent =
                formatDE(data.stats.co2_avg, 0);
        }

        const charts = [
            [cTemp, data.temperature],
            [cHum, data.humidity],
            [cPress, data.pressure],
            [cCO2, data.co2]
        ];

        charts.forEach(([chart, values]) => {
            if (!chart) return;

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

function downloadCSV() {
    const timeframe = document.getElementById("timeframe");
    const hours = timeframe ? timeframe.value : "24";

    window.location.href =
        `/api/export/csv?hours=${encodeURIComponent(hours)}`;
}

document.addEventListener("DOMContentLoaded", () => {
    cTemp = createChart(
        "chartTemp",
        "Temperatur (°C)",
        "#ff6b6b"
    );

    cHum = createChart(
        "chartHum",
        "Luftfeuchte (% rF)",
        "#4dabf7"
    );

    cPress = createChart(
        "chartPress",
        "Luftdruck (hPa)",
        "#cc5de8"
    );

    cCO2 = createChart(
        "chartCO2",
        "CO₂ (ppm)",
        "#51cf66"
    );

    document.getElementById("timeframe").addEventListener(
        "change",
        updateDashboard
    );

    document.getElementById("export-button").addEventListener(
        "click",
        downloadCSV
    );

    checkForNewData();

    setInterval(checkForNewData, POLL_INTERVAL);
});