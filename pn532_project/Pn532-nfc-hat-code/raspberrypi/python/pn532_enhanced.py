"""
Enhanced PN532 UART interface with robust error handling.
"""
import binascii
import time
import serial
import sys
import threading


class PN532:
    """PN532 NFC reader class with robust error handling."""
    
    def __init__(self, uart_port='/dev/ttyAMA0', baudrate=115200):
        """Initialize the PN532 reader."""
        self.uart = serial.Serial(uart_port, baudrate)
        self.debug = True
        
        # Clear any pending data
        if self.uart.in_waiting:
            self.uart.read(self.uart.in_waiting)
    
    def send_command(self, hex_str, retries=3):
        """Send a command as hex string with retries."""
        # Convert hex string to bytes
        if isinstance(hex_str, str):
            hex_str = hex_str.replace(' ', '').replace('0x', '').replace(',', '')
            command = binascii.unhexlify(hex_str)
        else:
            command = hex_str
            
        if self.debug:
            print('Sending:', ' '.join([f'{b:02X}' for b in command]))
            
        # Clear input buffer
        if self.uart.in_waiting:
            self.uart.read(self.uart.in_waiting)
            
        # Send command with retries
        for attempt in range(retries):
            try:
                self.uart.write(command)
                time.sleep(0.1)  # Wait for response
                
                if self.uart.in_waiting:
                    response = self.uart.read(self.uart.in_waiting)
                    if self.debug:
                        print(f'Response (attempt {attempt+1}): {" ".join([f"{b:02X}" for b in response])}')
                    return response
                else:
                    if self.debug and attempt < retries - 1:
                        print(f'No response on attempt {attempt+1}, retrying...')
                    time.sleep(0.1 * (attempt + 1))  # Increasing delay between retries
            except Exception as e:
                if self.debug:
                    print(f'Error on attempt {attempt+1}: {e}')
                if attempt == retries - 1:
                    raise
                time.sleep(0.1 * (attempt + 1))
            
        if self.debug:
            print('No response after all retries')
        return None
    
    def wake_up(self):
        """Send wake-up command to PN532."""
        wake_cmd = "55 55 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF 03 FD D4 14 01 17 00"
        response = self.send_command(wake_cmd)
        
        if response and len(response) >= 6:
            if self.debug:
                print("PN532 woke up successfully")
            return True
        return False
    
    def get_firmware_version(self):
        """Get PN532 firmware version."""
        firmware_cmd = "00 00 FF 02 FE D4 02 2A 00"
        response = self.send_command(firmware_cmd)
        
        if response and len(response) >= 12:
            # Parse firmware version (typical response format)
            if self.debug:
                try:
                    ic = response[7] if len(response) > 7 else 0
                    ver = response[8] if len(response) > 8 else 0
                    rev = response[9] if len(response) > 9 else 0
                    support = response[10] if len(response) > 10 else 0
                    print(f"Firmware: IC={ic:02X}, Version={ver}.{rev}, Support={support:02X}")
                except IndexError:
                    print("Couldn't parse firmware version")
            return response
        return None
    
    def configure_sam(self):
        """Configure the Secure Access Module (SAM)."""
        sam_cmd = "00 00 FF 05 FB D4 14 01 00 00 E9 00"
        response = self.send_command(sam_cmd)
        
        if response and len(response) >= 6:
            if self.debug:
                print("SAM configured successfully")
            return True
        return False
    
    def scan_for_card(self):
        """Scan for an NFC card."""
        scan_cmd = "00 00 FF 04 FC D4 4A 01 00 E1 00"
        response = self.send_command(scan_cmd)
        
        if response:
            # Try to find card data in response
            for i in range(len(response) - 9):
                if (i + 8 < len(response) and 
                    response[i:i+3] == b'\x00\x00\xFF' and
                    i + 6 < len(response) and response[i+6] == 0xD5 and
                    i + 7 < len(response) and response[i+7] == 0x4B and
                    i + 8 < len(response) and response[i+8] > 0):
                    
                    # Found a card response
                    if i + 12 < len(response):
                        uid_len = response[i+12]
                        
                        if i + 13 + uid_len <= len(response):
                            uid = response[i+13:i+13+uid_len]
                            if self.debug:
                                print(f"Card UID: {' '.join([f'{b:02X}' for b in uid])}")
                            return uid
            
            # If we got a response but couldn't parse it
            if self.debug:
                print("Got response but couldn't find card data")
        
        return None
    
    def close(self):
        """Close the UART connection."""
        self.uart.close()


class InteractiveMode:
    """Interactive mode for manual command sending and receiving."""
    
    def __init__(self, uart_port='/dev/ttyAMA0', baudrate=115200):
        """Initialize interactive mode."""
        self.uart = serial.Serial(uart_port, baudrate)
        
        # Clear any pending data
        if self.uart.in_waiting:
            self.uart.read(self.uart.in_waiting)
    
    def start(self):
        """Start interactive mode with read and write threads."""
        print('''
Interactive PN532 UART Mode

Usage:
Enter hex values to send, e.g.:
    55 55 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF 03 FD D4 14 01 17 00

Common commands:
- Wake up:            55 55 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF 03 FD D4 14 01 17 00
- Firmware version:   00 00 FF 02 FE D4 02 2A 00
- SAM configuration:  00 00 FF 05 FB D4 14 01 00 00 E9 00
- Scan for card:      00 00 FF 04 FC D4 4A 01 00 E1 00

Press Ctrl+C to quit
''')
        
        # Start read and write threads
        threads = []
        threads.append(threading.Thread(target=self._uart_write))
        threads.append(threading.Thread(target=self._uart_read))
        
        for thread in threads:
            thread.daemon = True
            thread.start()
        
        # Keep main thread alive until Ctrl+C
        try:
            while True:
                time.sleep(0.1)
        except KeyboardInterrupt:
            print("\nExiting...")
        finally:
            self.uart.close()
    
    def _uart_read(self):
        """Thread to read from UART."""
        while True:
            if self.uart.in_waiting:
                result = self.uart.read(self.uart.in_waiting)
                if result:
                    print('RX:', ' '.join([f'{i:02X}' for i in result]))
            time.sleep(0.05)
    
    def _uart_write(self):
        """Thread to write to UART."""
        while True:
            try:
                content = input()
                if not content.strip():
                    continue
                    
                # Handle special commands
                if content.lower() == 'help':
                    print('''
Common commands:
- Wake up:            55 55 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF 03 FD D4 14 01 17 00
- Firmware version:   00 00 FF 02 FE D4 02 2A 00
- SAM configuration:  00 00 FF 05 FB D4 14 01 00 00 E9 00
- Scan for card:      00 00 FF 04 FC D4 4A 01 00 E1 00
''')
                    continue
                    
                # Process hex string
                content = content.replace(' ', '').replace('0x', '').replace(',', '')
                try:
                    content = binascii.unhexlify(content)
                    print('TX:', ' '.join([f'{i:02X}' for i in content]))
                    self.uart.write(content)
                except Exception as e:
                    print(f"Error: {e}")
            except Exception as e:
                print(f"Input error: {e}")
            time.sleep(0.05)


def auto_test():
    """Perform automated testing of the PN532."""
    print("PN532 Automated Test\n")
    
    pn532 = PN532()
    
    try:
        # Step 1: Wake up the PN532
        print("\n[1/4] Testing wake-up sequence...")
        if pn532.wake_up():
            print("✓ Wake-up successful")
        else:
            print("✗ Wake-up failed")
        
        # Step 2: Get firmware version
        print("\n[2/4] Getting firmware version...")
        firmware = pn532.get_firmware_version()
        if firmware:
            print("✓ Firmware version received")
        else:
            print("✗ Failed to get firmware version")
        
        # Step 3: Configure SAM
        print("\n[3/4] Configuring SAM...")
        if pn532.configure_sam():
            print("✓ SAM configuration successful")
        else:
            print("✗ SAM configuration failed")
        
        # Step 4: Scan for cards
        print("\n[4/4] Testing card scanning...")
        print("Please place a card near the reader...")
        
        attempts = 0
        success = False
        
        while attempts < 5 and not success:
            attempts += 1
            print(f"Scan attempt {attempts}/5...")
            
            uid = pn532.scan_for_card()
            if uid:
                print(f"✓ Card detected! UID: {' '.join([f'{b:02X}' for b in uid])}")
                success = True
            else:
                print("✗ No card detected")
                time.sleep(1)
        
        # Final results
        print("\nTest Results:")
        print("=============")
        print("Wake-up:      " + ("PASS" if pn532.wake_up() else "FAIL"))
        print("Firmware:     " + ("PASS" if firmware else "FAIL"))
        print("SAM Config:   " + ("PASS" if pn532.configure_sam() else "FAIL"))
        print("Card Reading: " + ("PASS" if success else "FAIL"))
        
    finally:
        pn532.close()


def card_scanner():
    """Run continuous card scanning mode."""
    print("PN532 Card Scanner\n")
    
    pn532 = PN532()
    
    try:
        # Initialize the PN532
        print("Initializing PN532...")
        if not pn532.wake_up():
            print("Failed to wake up PN532. Exiting.")
            return
        
        time.sleep(0.5)
        
        firmware = pn532.get_firmware_version()
        if not firmware:
            print("Failed to get firmware version. Continuing anyway...")
        
        time.sleep(0.5)
        
        if not pn532.configure_sam():
            print("Failed to configure SAM. Continuing anyway...")
        
        # Start continuous scanning
        print("\nReady to scan cards. Place a card near the reader.")
        print("Press Ctrl+C to exit.\n")
        
        previous_uid = None
        
        while True:
            uid = pn532.scan_for_card()
            
            if uid:
                # Only show if it's a different card or same card after delay
                if uid != previous_uid:
                    print(f"\nCard detected! UID: {' '.join([f'{b:02X}' for b in uid])}")
                    previous_uid = uid
                    time.sleep(1)  # Delay to avoid repeated detections
            else:
                sys.stdout.write('.')
                sys.stdout.flush()
                previous_uid = None
                time.sleep(0.5)
    
    except KeyboardInterrupt:
        print("\nExiting...")
    finally:
        pn532.close()


if __name__ == '__main__':
    print("PN532 UART Interface")
    print("====================")
    print("1. Interactive Mode")
    print("2. Automated Test")
    print("3. Card Scanner")
    print("4. Exit")
    
    choice = input("\nSelect an option (1-4): ")
    
    if choice == '1':
        InteractiveMode().start()
    elif choice == '2':
        auto_test()
    elif choice == '3':
        card_scanner()
    else:
        print("Exiting...")
