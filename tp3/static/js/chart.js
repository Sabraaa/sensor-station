document.addEventListener('DOMContentLoaded', () => {
    let charts = {};
    const updateInterval = 10000; // Alle 10 Sekunden aktualisieren

    // Initialisierung aller Chart.js-Diagramme
    function initCharts() {
        const commonOptions = {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                x: {
                    grid: { display: false }
                },
                y: {
                    beginAtZero: false
                }
            },
            plugins: {
                legend: { display: false }
            }
        };

        charts.temp = new Chart(document.getElementById('chartTemp').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#e74c3c', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'Temperatur (°C)' } } }
        });

        charts.hum = new Chart(document.getElementById('chartHum').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#3498db', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'Luftfeuchte (% rF)' } } }
        });

        charts.press = new Chart(document.getElementById('chartPress').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#9b59b6', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'Luftdruck (hPa)' } } }
        });

        charts.co2 = new Chart(document.getElementById('chartCO2').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#2ecc71', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'CO₂ (ppm)' } } }
        });

        charts.nh2 = new Chart(document.getElementById('chartNH2').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#e67e22', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'NH₃ / NH₂ (ppm)' } } }
        });

        charts.co = new Chart(document.getElementById('chartCO').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#f1c40f', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'CO (ppm)' } } }
        });

        charts.n2 = new Chart(document.getElementById('chartN2').getContext('2d'), {
            type: 'line',
            data: { labels: [], datasets: [{ data: [], borderColor: '#1abc9c', tension: 0.2, fill: false }] },
            options: { ...commonOptions, plugins: { ...commonOptions.plugins, title: { display: true, text: 'N₂ (%)' } } }
        });
    }

    // Aktuellen Messwert abrufen und Kacheln aktualisieren
    async function fetchCurrentData() {
        try {
            const response = await fetch('/api/current');
            if (!response.ok) throw new Error('Netzwerk-Antwort war nicht ok');
            const data = await response.json();

            document.getElementById('val-temp').innerText = data.temperature !== null ? data.temperature.toFixed(1) : '--';
            document.getElementById('val-hum').innerText = data.humidity !== null ? data.humidity.toFixed(1) : '--';
            document.getElementById('val-press').innerText = data.pressure !== null ? data.pressure.toFixed(1) : '--';
            document.getElementById('val-co2').innerText = data.co2 !== null ? data.co2 : '--';
            document.getElementById('val-nh2').innerText = data.nh3 !== null ? data.nh3.toFixed(2) : '--';
            document.getElementById('val-co').innerText = data.co !== null ? data.co.toFixed(2) : '--';
            document.getElementById('val-n2').innerText = data.n2 !== null ? data.n2.toFixed(1) : '--';

            document.getElementById('last-update').innerText = data.server_time;
            document.getElementById('connection-status').innerText = 'Verbunden';
            document.getElementById('connection-status').style.color = 'green';
        } catch (error) {
            console.error('Fehler beim Abrufen der aktuellen Daten:', error);
            document.getElementById('connection-status').innerText = 'Fehler (Keine Verbindung)';
            document.getElementById('connection-status').style.color = 'red';
        }
    }

    // Historische Daten für die Charts und Statistiken abrufen
    async function fetchHistoryData() {
        const hours = document.getElementById('timeframe').value;
        document.getElementById('stats-label').innerText = hours == 1 ? '1h' : (hours == 24 ? '24h' : '7T');

        try {
            const response = await fetch(`/api/history?hours=${hours}`);
            if (!response.ok) throw new Error('Fehler beim Laden der Historie');
            const data = await response.json();

            // Labels (Zeitstempel) für alle Charts setzen
            const labels = data.timestamps;

            updateChart(charts.temp, labels, data.temperature);
            updateChart(charts.hum, labels, data.humidity);
            updateChart(charts.press, labels, data.pressure);
            updateChart(charts.co2, labels, data.co2);
            updateChart(charts.nh2, labels, data.nh3);
            updateChart(charts.co, labels, data.co);
            updateChart(charts.n2, labels, data.n2);

            // Statistik-Werte aktualisieren
            if (data.stats) {
                document.getElementById('st-t-min').innerText = data.stats.temp_min ?? '--';
                document.getElementById('st-t-max').innerText = data.stats.temp_max ?? '--';
                document.getElementById('st-t-avg').innerText = data.stats.temp_avg ?? '--';

                document.getElementById('st-c-min').innerText = data.stats.co2_min ?? '--';
                document.getElementById('st-c-max').innerText = data.stats.co2_max ?? '--';
                document.getElementById('st-c-avg').innerText = data.stats.co2_avg ?? '--';
            }

        } catch (error) {
            console.error('Fehler beim Verarbeiten der historischen Daten:', error);
        }
    }

    function updateChart(chart, labels, dataValues) {
        chart.data.labels = labels;
        chart.data.datasets[0].data = dataValues;
        chart.update('none'); // 'none' für flüssiges Update ohne störende Animationen beim Intervall
    }

    // Event-Listener für Zeitraum-Änderung
    document.getElementById('timeframe').addEventListener('change', fetchHistoryData);

    // Event-Listener für CSV-Export-Button
    document.getElementById('export-button').addEventListener('click', () => {
        const hours = document.getElementById('timeframe').value;
        window.location.href = `/api/export/csv?hours=${hours}`;
    });

    // Initialisierung starten
    initCharts();
    fetchCurrentData();
    fetchHistoryData();

    // Regelmäßiges Aktualisieren im Hintergrund
    setInterval(() => {
        fetchCurrentData();
    }, updateInterval);
});