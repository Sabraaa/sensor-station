from flask import Flask, render_template, jsonify, request, Response, make_response
from datetime import datetime
import mysql.connector
import json
import csv
import io

app = Flask(__name__)

def get_db_connection():
    with open('config.json') as f:
        config = json.load(f)
    return mysql.connector.connect(
        host=config['db_host'],
        user=config['db_readonly_user'],
        password=config['db_readonly_password'],
        database=config['db_name']
    )

@app.route('/')
def index():
    return render_template('index.html')

@app.route('/api/current', methods=['GET'])
def get_current():
    try:
        conn = get_db_connection()
        cursor = conn.cursor(dictionary=True)
        
        query = """
            SELECT timestamp, temperature, humidity, pressure, co2, gas_raw 
            FROM measurements 
            ORDER BY timestamp DESC 
            LIMIT 1
        """
        cursor.execute(query)
        data = cursor.fetchone()
        
        cursor.close()
        conn.close()

        if not data:
            return make_response(jsonify({"error": "No data available"}), 503)

        current_server_time = datetime.now().isoformat(timespec='seconds')
        timestamp_str = data['timestamp'].isoformat(timespec='seconds') if isinstance(data['timestamp'], datetime) else str(data['timestamp'])

        response_payload = {
            "timestamp": timestamp_str,
            "server_time": current_server_time,
            "temperature": float(data['temperature']) if data['temperature'] is not None else None,
            "humidity": float(data['humidity']) if data['humidity'] is not None else None,
            "pressure": float(data['pressure']) if data['pressure'] is not None else None,
            "co2": int(data['co2']) if data['co2'] is not None else None,
            "gas_raw": int(data['gas_raw']) if data['gas_raw'] is not None else None
        }

        return jsonify(response_payload), 200

    except Exception as e:
        return make_response(jsonify({"error": "Database error", "details": str(e)}), 503)

@app.route('/api/history', methods=['GET'])
def get_history():
    hours = request.args.get('hours', default=24, type=int)
    
    try:
        conn = get_db_connection()
        cursor = conn.cursor(dictionary=True)
        
        query = """
            SELECT timestamp, temperature, humidity, pressure, co2, gas_raw 
            FROM measurements 
            WHERE timestamp >= NOW() - INTERVAL %s HOUR
            ORDER BY timestamp ASC
        """
        cursor.execute(query, (hours,))
        rows = cursor.fetchall()
        
        cursor.close()
        conn.close()

        timestamps, temps, hums, press, co2_vals = [], [], [], [], []
        
        for r in rows:
            timestamps.append(r['timestamp'].strftime('%d.%m.%Y %H:%M:%S'))
            temps.append(float(r['temperature']) if r['temperature'] is not None else None)
            hums.append(float(r['humidity']) if r['humidity'] is not None else None)
            press.append(float(r['pressure']) if r['pressure'] is not None else None)
            co2_vals.append(int(r['co2']) if r['co2'] is not None else None)

        valid_temps = [t for t in temps if t is not None]
        valid_co2 = [c for c in co2_vals if c is not None]

        stats = {
            "temp_min": round(min(valid_temps), 1) if valid_temps else None,
            "temp_max": round(max(valid_temps), 1) if valid_temps else None,
            "temp_avg": round(sum(valid_temps) / len(valid_temps), 1) if valid_temps else None,
            "co2_min": min(valid_co2) if valid_co2 else None,
            "co2_max": max(valid_co2) if valid_co2 else None,
            "co2_avg": int(sum(valid_co2) / len(valid_co2)) if valid_co2 else None
        }

        return jsonify({
            "timestamps": timestamps,
            "temperature": temps,
            "humidity": hums,
            "pressure": press,
            "co2": co2_vals,
            "stats": stats
        }), 200

    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/export/csv', methods=['GET'])
def export_csv():
    hours = request.args.get('hours', default=24, type=int)
    
    try:
        conn = get_db_connection()
        cursor = conn.cursor(dictionary=True)
        query = """
            SELECT timestamp, temperature, humidity, pressure, co2, gas_raw 
            FROM measurements 
            WHERE timestamp >= NOW() - INTERVAL %s HOUR
            ORDER BY timestamp ASC
        """
        cursor.execute(query, (hours,))
        rows = cursor.fetchall()
        cursor.close()
        conn.close()

        output = io.StringIO()
        writer = csv.writer(output, delimiter=';')
        writer.writerow(['Zeitstempel', 'Temperatur (C)', 'Feuchte (% rF)', 'Luftdruck (hPa)', 'CO2 (ppm)', 'Gas Raw'])

        for r in rows:
            writer.writerow([
                r['timestamp'].strftime('%Y-%m-%d %H:%M:%S'),
                str(r['temperature']).replace('.', ','),
                str(r['humidity']).replace('.', ','),
                str(r['pressure']).replace('.', ','),
                r['co2'],
                r['gas_raw']
            ])

        return Response(
            output.getvalue(),
            mimetype="text/csv",
            headers={"Content-disposition": f"attachment; filename=umweltmesswerte_{hours}h.csv"}
        )

    except Exception as e:
        return jsonify({"error": str(e)}), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000, debug=False)