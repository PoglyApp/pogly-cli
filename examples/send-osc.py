#!/usr/bin/env python3
"""
Example script showing how to send OSC (Open Sound Control) messages
to control a Pogly overlay via the pogly-cli OSC listener.

Requirements:
    pip install python-osc

Usage:
    1. Start the listener:
       pogly osc --port 9000

    2. Run this script:
       python send-osc.py
"""

import time
import argparse
from pythonosc import udp_client

def main():
    parser = argparse.ArgumentParser(description="Send OSC messages to pogly-cli")
    parser.add_argument("--host", default="127.0.0.1", help="The host running pogly osc")
    parser.add_argument("--port", type=int, default=9000, help="The port pogly osc is listening on")
    args = parser.parse_args()

    client = udp_client.SimpleUDPClient(args.host, args.port)

    print(f"Sending OSC messages to {args.host}:{args.port}...")

    # 1. Ping the listener
    print("Sending ping...")
    client.send_message("/pogly/ping", [])
    time.sleep(1)

    # 2. Switch to active layout by name (replace 'Main' with your layout name if different)
    print("Switching layout to 'Main'...")
    client.send_message("/pogly/layouts/set-active", "Main")
    time.sleep(1)

    # 3. Update an element's position (replace 42 with your element ID)
    element_id = 42
    print(f"Moving element {element_id} to (100, 200)...")
    client.send_message("/pogly/elements/update/position", [element_id, 100, 200])
    time.sleep(1)

    # 4. Fade element opacity to 50%
    print(f"Setting element {element_id} transparency to 50%...")
    client.send_message("/pogly/elements/update/transparency", [element_id, 50])
    time.sleep(1)

    # 5. Generic field update: change text color to green (#00ff00)
    print(f"Changing element {element_id} text color to green...")
    client.send_message("/pogly/elements/update", [element_id, "color", "#00ff00"])
    
    print("Done!")

if __name__ == "__main__":
    main()
