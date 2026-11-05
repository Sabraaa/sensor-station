import mariadb
import time
import sys
import serial
import mysql.connector

def get_db_connection():
    with open('config.json') as f:
        config = json.load(f)
    return mysql.connector.connect(
        host=config['db_host'],
        user=config['db_readonly_user'],
        password=config['db_readonly_password'],
        database=config['db_name']
    )