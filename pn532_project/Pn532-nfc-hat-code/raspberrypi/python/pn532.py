"""
This module contains classes to interact with the PN532 NFC chip.
"""

import RPi.GPIO as GPIO
import time
import binascii

# PN532 Commands
_COMMAND_DIAGNOSE = 0x00
_COMMAND_GETFIRMWAREVERSION = 0x02
_COMMAND_GETGENERALSTATUS = 0x04
_COMMAND_READREGISTER = 0x06
_COMMAND_WRITEREGISTER = 0x08
_COMMAND_READGPIO = 0x0C
_COMMAND_WRITEGPIO = 0x0E
_COMMAND_SETSERIALBAUDRATE = 0x10
_COMMAND_SETPARAMETERS = 0x12
_COMMAND_SAMCONFIGURATION = 0x14
_COMMAND_POWERDOWN = 0x16
_COMMAND_RFCONFIGURATION = 0x32
_COMMAND_RFREGULATIONTEST = 0x58
_COMMAND_INJUMPFORDEP = 0x56
_COMMAND_INJUMPFORPSL = 0x46
_COMMAND_INLISTPASSIVETARGET = 0x4A
_COMMAND_INATR = 0x50
_COMMAND_INPSL = 0x4E
_COMMAND_INDATAEXCHANGE = 0x40
_COMMAND_INCOMMUNICATETHRU = 0x42
_COMMAND_INDESELECT = 0x44
_COMMAND_INRELEASE = 0x52
_COMMAND_INSELECT = 0x54
_COMMAND_INAUTOPOLL = 0x60
_COMMAND_TGINITASTARGET = 0x8C
_COMMAND_TGSETGENERALBYTES = 0x92
_COMMAND_TGGETDATA = 0x86
_COMMAND_TGSETDATA = 0x8E
_COMMAND_TGSETMETADATA = 0x94
_COMMAND_TGGETINITIATORCOMMAND = 0x88
_COMMAND_TGRESPONSETOINITIATOR = 0x90
_COMMAND_TGGETTARGETSTATUS = 0x8A

_RESPONSE_INDATAEXCHANGE = 0x41
_RESPONSE_INLISTPASSIVETARGET = 0x4B

_WAKEUP = 0x55

_MIFARE_ISO14443A = 0x00

# Mifare Commands
MIFARE_CMD_AUTH_A = 0x60
MIFARE_CMD_AUTH_B = 0x61
MIFARE_CMD_READ = 0x30
MIFARE_CMD_WRITE = 0xA0
MIFARE_CMD_TRANSFER = 0xB0
MIFARE_CMD_DECREMENT = 0xC0
MIFARE_CMD_INCREMENT = 0xC1
MIFARE_CMD_STORE = 0xC2
MIFARE_ULTRALIGHT_CMD_WRITE = 0xA2

# NTAG 2xx Commands
NTAG2XX_BLOCK_COUNT = 231
NTAG2XX_PAGE_SIZE = 4
NTAG2XX_USER_START_PAGE = 4
NTAG2XX_USER_END_PAGE = 129
NTAG2XX_CMD_READ = 0x30
NTAG2XX_CMD_WRITE = 0xA2

# SPI Status Values
PN532_SPI_STATREAD = 0x02
PN532_SPI_DATAWRITE = 0x01
PN532_SPI_DATAREAD = 0x03
PN532_SPI_READY = 0xFF


class PN532(object):
    """PN532 base class"""

    def __init__(self, debug=False, reset=20):
        """Create an instance of the PN532 class
        
        Args:
            debug: boolean, Whether debug output is enabled
            reset: integer, Reset pin
        """
        self.debug = debug
        self._reset_pin = reset
        self._gpio = None
        self._gpio_setup = False

    def _gpio_init(self):
        """Initialize GPIO"""
        self._gpio = GPIO
        self._gpio.setmode(GPIO.BCM)
        self._gpio.setup(self._reset_pin, GPIO.OUT)
        self._gpio.output(self._reset_pin, GPIO.HIGH)
        self._gpio_setup = True

    def _reset(self):
        """Perform hardware reset of the PN532"""
        if self._reset_pin is not None:
            if not self._gpio_setup:
                self._gpio_init()
            self._gpio.output(self._reset_pin, GPIO.HIGH)
            time.sleep(0.1)
            self._gpio.output(self._reset_pin, GPIO.LOW)
            time.sleep(0.5)
            self._gpio.output(self._reset_pin, GPIO.HIGH)
            time.sleep(0.1)

    def firmware_version(self):
        """Call PN532 GetFirmwareVersion function and return firmware version."""
        return self.call_function(
            _COMMAND_GETFIRMWAREVERSION, params=None, response_length=4)

    def get_firmware_version(self):
        """Call PN532 GetFirmwareVersion function and return firmware version."""
        print("SENDING GET_FIRMWARE_VERSION COMMAND")
        response = self.call_function(
            _COMMAND_GETFIRMWAREVERSION, params=None, response_length=4)
        print("FIRMWARE RESPONSE:", [hex(i) for i in response] if response else "None")
        if response is None:
            raise RuntimeError('Failed to get firmware version!')
        return (response[0], response[1], response[2], response[3])

    def SAM_configuration(self):
        """Configure the PN532 to read MiFare cards."""
        return self.call_function(
            _COMMAND_SAMCONFIGURATION,
            params=[0x01, 0x14, 0x01],
            response_length=1)

    def read_passive_target(self, card_baud=_MIFARE_ISO14443A, timeout=1000):
        """Wait for a MiFare card to be available and return its UID when found.
        Will wait up to timeout (in millisecond) to see a card.
        """
        response = self.call_function(
            _COMMAND_INLISTPASSIVETARGET,
            params=[0x01, card_baud],
            response_length=20,
            timeout=timeout)
        
        if response is None:
            return None
            
        # Check only 1 card with up to a 7 byte UID is present.
        if response[0] != 0x01:
            raise RuntimeError('More than one card detected!')
        if response[5] > 7:
            raise RuntimeError('Found card with unexpectedly long UID!')
            
        return response[6:6+response[5]]

    def read_mifare(self, block_number, key_number=0, key=None):
        """Read a block of data from a mifare card

        Args:
            block_number: integer, The block to read
            key_number: integer, The key to use.
                        0 = MIFARE_CMD_AUTH_A
                        1 = MIFARE_CMD_AUTH_B
            key: array of 6 bytes containing the key

        Returns:
            array: 16 bytes of data

        Raises:
            RuntimeError
        """
        if not key:
            key = b'\xFF\xFF\xFF\xFF\xFF\xFF'
        uidlen, uid = self.get_passive_mifare()
        key_cmd = [MIFARE_CMD_AUTH_B, block_number]
        if key_number == 0:
            key_cmd[0] = MIFARE_CMD_AUTH_A
        key_cmd.extend(key)
        key_cmd.extend(uid)

        response = self.call_function(_COMMAND_INDATAEXCHANGE,
                       [0x01] + key_cmd,
                       response_length=1,
                       timeout=1000)
        
        if response[0] != 0:
            raise RuntimeError('Failed to authenticate card')
        
        response = self.call_function(_COMMAND_INDATAEXCHANGE,
                       [0x01, MIFARE_CMD_READ, block_number],
                       response_length=17,
                       timeout=1000)
        
        if response[0] != 0:
            raise RuntimeError('Failed to read data')
        
        return response[1:]
    
    def write_mifare(self, block_number, data, key_number=0, key=None):
        """Write a block of data to a mifare card

        Args:
            block_number: integer, The block to write
            data: array of 16 bytes of data to write
            key_number: integer, The key to use.
                        0 = MIFARE_CMD_AUTH_A
                        1 = MIFARE_CMD_AUTH_B
            key: array of 6 bytes containing the key

        Returns:
            boolean: Success or Failure

        Raises:
            RuntimeError
        """
        assert data and len(data) == 16, 'Data must be an array of 16 bytes!'

        if not key:
            key = b'\xFF\xFF\xFF\xFF\xFF\xFF'
        uidlen, uid = self.get_passive_mifare()
        key_cmd = [MIFARE_CMD_AUTH_B, block_number]
        if key_number == 0:
            key_cmd[0] = MIFARE_CMD_AUTH_A
        key_cmd.extend(key)
        key_cmd.extend(uid)

        response = self.call_function(_COMMAND_INDATAEXCHANGE,
                       [0x01] + key_cmd,
                       response_length=1,
                       timeout=1000)
        
        if response[0] != 0:
            raise RuntimeError('Failed to authenticate card')
        
        response = self.call_function(_COMMAND_INDATAEXCHANGE,
                       [0x01, MIFARE_CMD_WRITE, block_number] + data,
                       response_length=1,
                       timeout=1000)
        
        if response[0] != 0:
            raise RuntimeError('Failed to write data')
        
        return True
    
    def read_ntag2xx(self, block_number):
        """Read a block of data from a ntag2xx card

        Args:
            block_number: integer, The block to read

        Returns:
            array: 16 bytes of data

        Raises:
            RuntimeError
        """
        if block_number > NTAG2XX_BLOCK_COUNT:
            raise RuntimeError('Block must be 0..' + str(NTAG2XX_BLOCK_COUNT))

        response = self.call_function(_COMMAND_INDATAEXCHANGE,
                       [0x01, NTAG2XX_CMD_READ, block_number],
                       response_length=17,
                       timeout=1000)
        
        if response[0] != 0:
            raise RuntimeError('Failed to read data')
        
        return response[1:]
    
    def write_ntag2xx(self, block_number, data):
        """Write a block of data to a ntag2xx card

        Args:
            block_number: integer, The block to write
            data: array of 4 bytes of data to write

        Returns:
            boolean: Success or Failure

        Raises:
            RuntimeError
        """
        assert data and len(data) == 4, 'Data must be an array of 4 bytes!'
        if block_number < NTAG2XX_USER_START_PAGE or block_number > NTAG2XX_USER_END_PAGE:
            raise RuntimeError('Block must be user block' + str(NTAG2XX_USER_START_PAGE) + '..' + str(NTAG2XX_USER_END_PAGE))
        
        response = self.call_function(_COMMAND_INDATAEXCHANGE,
                       [0x01, NTAG2XX_CMD_WRITE, block_number] + data,
                       response_length=1,
                       timeout=1000)
        
        if response[0] != 0:
            raise RuntimeError('Failed to write data')
        
        return True
    
    def read_gpio(self):
        """Read the GPIO pins

        Returns:
            array: Two bytes containing the pin states

        Raises:
            RuntimeError
        """
        response = self.call_function(_COMMAND_READGPIO,
                       params=None,
                       response_length=3,
                       timeout=1000)
        
        if not response:
            raise RuntimeError('Failed to read GPIO')
        
        return response
    
    def write_gpio(self, p3, p7):
        """Write the GPIO pins

        Args:
            p3: byte, Bit values for P3 pins.
            p7: byte, Bit values for P7 pins.

        Returns:
            boolean: Success or Failure

        Raises:
            RuntimeError
        """
        response = self.call_function(_COMMAND_WRITEGPIO,
                       params=[p3, p7],
                       response_length=1,
                       timeout=1000)
        
        if not response:
            raise RuntimeError('Failed to write GPIO')
        
        if response[0] != 0:
            raise RuntimeError('Failed to write GPIO')
        
        return True
    
    def get_passive_mifare(self):
        """Wait for a MiFare card to be available and return its UID when found.

        Returns:
            tuple: uidlen and uid
        """
        response = self.read_passive_target()
        if response is None:
            return None
        
        return (len(response), response)


class PN532_SPI(PN532):
    """PN532 SPI"""

    def __init__(self, cs=4, reset=20, debug=False, spi=None):
        """Create an instance of the PN532 class using SPI"""
        super(PN532_SPI, self).__init__(debug=debug, reset=reset)
        self._cs = cs
        self._gpio = None
        self._spi = None
        self.cs_low = lambda: self._gpio.output(self._cs, GPIO.LOW)
        self.cs_high = lambda: self._gpio.output(self._cs, GPIO.HIGH)
        
        if not self._gpio_setup:
            self._gpio_init()
            
        self._gpio.setup(self._cs, GPIO.OUT)
        self._gpio.output(self._cs, GPIO.HIGH)
        
        if spi is None:
            import spidev
            self._spi = spidev.SpiDev()
            self._spi.open(0, 0)
            self._spi.max_speed_hz = 1000000
        else:
            self._spi = spi
            
        self._reset()
        if self.firmware_version() is None:
            raise RuntimeError("Failed to detect the PN532")

    def _wait_ready(self, timeout=1):
        """Poll PN532 if status byte is ready, up to `timeout` seconds"""
        status = 0
        start = time.time()
        while ((time.time() - start) < timeout):
            self._gpio.output(self._cs, GPIO.LOW)
            status = self._spi.xfer2([PN532_SPI_STATREAD, 0])[1]
            self._gpio.output(self._cs, GPIO.HIGH)
            print(f"STATUS CHECK: 0x{status:02X}")
            if status == 0x01:  # Not ready yet
                time.sleep(0.01)
                continue
            return status == 0xFF  # 0xFF = ready
        print("STATUS CHECK TIMEOUT")
        return False

    def call_function(self, command, params=[], response_length=0, timeout=1):
        """Send specified command to the PN532 and expect up to response_length bytes back.
        Note that less than the expected bytes might be returned!
        """
        # Build frame data with command and parameters
        data = bytearray(2 + len(params))
        data[0] = 0xD4
        data[1] = command & 0xFF
        for i, val in enumerate(params):
            data[2+i] = val
        
        # Send frame
        try:
            self.send_command(data)
        except OSError:
            print("Warning: OSError in send_command(), retrying...")
            self._wakeup()
            return None
        
        # Wait for ACK
        if not self._wait_ready(timeout):
            print("Timed out waiting for ACK")
            return None
            
        # Read ACK
        self._gpio.output(self._cs, GPIO.LOW)
        ack = self._spi.xfer2([PN532_SPI_DATAREAD, 0, 0, 0, 0, 0, 0])[1:]
        self._gpio.output(self._cs, GPIO.HIGH)
        
        print("ACK RESPONSE:", [hex(i) for i in ack])
        
        # Check ACK
        if ack[0] != 0 or ack[1] != 0 or ack[2] != 0xFF:
            print("Invalid ACK")
            return None
        
        # Wait for response
        if not self._wait_ready(timeout):
            print("Timed out waiting for response")
            return None
            
        # Read response
        self._gpio.output(self._cs, GPIO.LOW)
        frame = self._spi.xfer2([PN532_SPI_DATAREAD, 0, 0, 0])[1:]
        self._gpio.output(self._cs, GPIO.HIGH)
        
        print("RESPONSE HEADER:", [hex(i) for i in frame])
        
        # Check frame headers
        if frame[0] != 0 or frame[1] != 0 or frame[2] != 0xFF:
            print("Invalid frame header")
            return None
            
        # Check length & length checksum match
        data_len = frame[3]
        if (data_len + frame[4]) & 0xFF != 0:
            print("Invalid length checksum")
            return None
            
        # Read remaining bytes in frame
        max_read = min(response_length+6, data_len+6)
        self._gpio.output(self._cs, GPIO.LOW)
        response = self._spi.xfer2([PN532_SPI_DATAREAD] + [0]*(max_read-4))[1:]
        self._gpio.output(self._cs, GPIO.HIGH)
        
        print("FULL RESPONSE:", [hex(i) for i in frame + response])
        
        # Check frame checksum value matches bytes
        checksum = 0
        for i in range(data_len + 1):
            checksum += response[5+i]
        if checksum & 0xFF != 0:
            print("Invalid checksum")
            return None
            
        # Return frame data
        return response[6:6+data_len]

    def send_command(self, data):
        """Send a command frame to the PN532"""
        print("SENDING COMMAND:", [hex(i) for i in data])
        assert data is not None and 1 < len(data) < 255, 'Data must be array of 1 to 255 bytes.'
        
        # Build command frame
        cmd = bytearray(len(data) + 8)
        cmd[0] = 0x00  # Preamble
        cmd[1] = 0x00  # Start code
        cmd[2] = 0xFF  # Start code
        cmd[3] = len(data) + 1  # Length of data + TFI
        cmd[4] = (~cmd[3] + 1) & 0xFF  # Checksum of length
        cmd[5:5+len(data)] = data  # Data
        
        # Calculate checksum
        checksum = sum(data) & 0xFF
        cmd[5+len(data)] = ~checksum & 0xFF  # Checksum of data
        cmd[6+len(data)] = 0x00  # Postamble
        
        # Send command
        self.write_data(bytes(cmd))

    def write_data(self, data):
        """Write a command to the PN532"""
        print("SPI WRITE:", [hex(i) for i in data])
        self._gpio.output(self._cs, GPIO.LOW)
        self._spi.writebytes([PN532_SPI_DATAWRITE] + list(data))
        self._gpio.output(self._cs, GPIO.HIGH)
        print("SPI WRITE COMPLETE")

    def read_data(self, count):
        """Read response data from the PN532"""
        print(f"SPI READ ATTEMPT (expecting {count} bytes)")
        self._gpio.output(self._cs, GPIO.LOW)
        
        # Send a read command
        response = self._spi.xfer2([PN532_SPI_DATAREAD] + [0] * (count + 1))[1:]
        self._gpio.output(self._cs, GPIO.HIGH)
        print("SPI READ RESPONSE:", [hex(i) for i in response])
        return response

    def _wakeup(self):
        """Send any special commands/data to wake up PN532"""
        time.sleep(0.01)
        self._gpio.output(self._cs, GPIO.LOW)
        self._spi.writebytes([0x00])
        time.sleep(0.01)
        self._gpio.output(self._cs, GPIO.HIGH)


class PN532_I2C(PN532):
    """PN532 I2C"""

    def __init__(self, irq=None, reset=None, req=None, addr=0x24, debug=False, i2c=None):
        """Create an instance of the PN532 class using I2C"""
        super(PN532_I2C, self).__init__(debug=debug, reset=reset)
        self._addr = addr
        self._irq = irq
        self._req = req
        self._i2c = None
        
        if i2c is None:
            import smbus
            self._i2c = smbus.SMBus(1)
        else:
            self._i2c = i2c
            
        if self._irq is not None:
            self._gpio.setup(self._irq, GPIO.IN)
            
        if self._req is not None:
            self._gpio.setup(self._req, GPIO.OUT)
            self._gpio.output(self._req, GPIO.HIGH)
            
        self._reset()
        if self.firmware_version() is None:
            raise RuntimeError("Failed to detect the PN532")
    
    def _wait_ready(self, timeout=1):
        """Wait for chip to be ready by IRQ or polling"""
        start = time.time()
        
        # IRQ pin
        if self._irq is not None:
            while self._gpio.input(self._irq):
                if time.time() - start > timeout:
                    return False
                time.sleep(0.01)
            return True
        
        # Polling
        while time.time() - start < timeout:
            try:
                status = self._i2c.read_byte(self._addr)
                if status & 1:
                    return True
            except OSError:
                pass
            time.sleep(0.01)
        return False

    def call_function(self, command, params=[], response_length=0, timeout=1):
        """Send specified command to the PN532 and expect up to response_length bytes back.
        Note that less than the expected bytes might be returned!
        """
        # Build frame data with command and parameters
        data = bytearray(2 + len(params))
        data[0] = 0xD4
        data[1] = command & 0xFF
        for i, val in enumerate(params):
            data[2+i] = val
        
        # Send frame
        try:
            self.send_command(data)
        except OSError:
            print("Warning: OSError in send_command(), retrying...")
            self._wakeup()
            return None
        
        # Wait for ACK
        if not self._wait_ready(timeout):
            print("Timed out waiting for ACK")
            return None
            
        # Read ACK
        try:
            ack = self.read_data(6)
        except OSError:
            print("Warning: OSError in read_data()")
            return None
            
        # Check ACK
        if ack[0] != 0 or ack[1] != 0 or ack[2] != 0xFF:
            print("Invalid ACK")
            return None
        
        # Wait for response
        if not self._wait_ready(timeout):
            print("Timed out waiting for response")
            return None
            
        # Read response
        try:
            frame = self.read_data(3)
        except OSError:
            print("Warning: OSError in read_data()")
            return None
            
        # Check frame headers
        if frame[0] != 0 or frame[1] != 0 or frame[2] != 0xFF:
            print("Invalid frame header")
            return None
            
        # Read length and check it's valid
        frame_len = self.read_data(2)
        if (frame_len[0] + frame_len[1]) & 0xFF != 0:
            print("Invalid length checksum")
            return None
            
        # Read the rest of the frame
        data_len = frame_len[0]
        try:
            response = self.read_data(data_len + 2)
        except OSError:
            print("Warning: OSError in read_data()")
            return None
            
        # Check checksum
        checksum = 0
        for i in range(data_len + 1):
            checksum += response[i]
        if checksum & 0xFF != 0:
            print("Invalid checksum")
            return None
            
        # Return data
        return response[1:1+data_len-1]

    def send_command(self, data):
        """Send a command frame to the PN532"""
        assert data is not None and 1 < len(data) < 255, 'Data must be array of 1 to 255 bytes.'
        
        # Build command frame
        cmd = bytearray(len(data) + 7)
        cmd[0] = 0x00  # Preamble
        cmd[1] = 0x00  # Start code
        cmd[2] = 0xFF  # Start code
        cmd[3] = len(data) + 1  # Length of data + TFI
        cmd[4] = (~cmd[3] + 1) & 0xFF  # Checksum of length
        cmd[5:5+len(data)] = data  # Data
        
        # Calculate checksum
        checksum = sum(data) & 0xFF
        cmd[5+len(data)] = ~checksum & 0xFF  # Checksum of data
        
        # Send command
        if self._req is not None:
            self._gpio.output(self._req, GPIO.LOW)
        self.write_data(bytes(cmd))
        if self._req is not None:
            self._gpio.output(self._req, GPIO.HIGH)

    def write_data(self, data):
        """Write a command to the PN532"""
        for i in range(0, len(data), 16):
            chunk = data[i:min(i+16, len(data))]
            for b in range(len(chunk)):
                self._i2c.write_byte(self._addr, chunk[b])

    def read_data(self, count):
        """Read response data from the PN532"""
        result = bytearray(count)
        for i in range(count):
            result[i] = self._i2c.read_byte(self._addr)
        return result

    def _wakeup(self):
        """Send any special commands/data to wake up PN532"""
        if self._req is not None:
            self._gpio.output(self._req, GPIO.LOW)
            time.sleep(0.01)
            self._gpio.output(self._req, GPIO.HIGH)
            time.sleep(0.01)


class PN532_UART(PN532):
    """PN532 UART"""

    def __init__(self, uart_id=None, baudrate=115200, reset=20, debug=False):
        """Create an instance of the PN532 class using I2C"""
        super(PN532_UART, self).__init__(debug=debug, reset=reset)
        self._uart = None
        
        import serial
        if uart_id is not None:
            self._uart = serial.Serial(uart_id, baudrate=baudrate)
        else:
            self._uart = serial.Serial('/dev/ttyS0', baudrate=baudrate)
            
        self._reset()
        if self.firmware_version() is None:
            raise RuntimeError("Failed to detect the PN532")

    def _wait_ready(self, timeout=1):
        """Wait for response"""
        timeout *= 1000
        start = time.time() * 1000
        while (time.time() * 1000) - start < timeout:
            if self._uart.in_waiting:
                return True
            time.sleep(0.01)
        return False

    def call_function(self, command, params=[], response_length=0, timeout=1):
        """Send specified command to the PN532 and expect up to response_length bytes back.
        Note that less than the expected bytes might be returned!
        """
        # Build frame data with command and parameters
        data = bytearray(2 + len(params))
        data[0] = 0xD4
        data[1] = command & 0xFF
        for i, val in enumerate(params):
            data[2+i] = val
        
        # Send frame
        try:
            self.send_command(data)
        except RuntimeError:
            print("Warning: Error in send_command(), retrying...")
            self._wakeup()
            return None
        
        # Wait for response
        if not self._wait_ready(timeout):
            print("Timed out waiting for ACK")
            return None
            
        # Read ACK
        ack = self.read_data(6)
        
        # Check ACK
        if ack[0] != 0 or ack[1] != 0 or ack[2] != 0xFF:
            print("Invalid ACK")
            return None
        
        # Wait for response
        if not self._wait_ready(timeout):
            print("Timed out waiting for response")
            return None
            
        # Read response
        header = self.read_data(3)
        
        # Check response headers
        if header[0] != 0 or header[1] != 0 or header[2] != 0xFF:
            print("Invalid frame header")
            return None
            
        # Read length and check it's valid
        frame_len = self.read_data(2)
        if (frame_len[0] + frame_len[1]) & 0xFF != 0:
            print("Invalid length checksum")
            return None
            
        # Read the rest of the frame
        data_len = frame_len[0]
        response = self.read_data(data_len + 2)
            
        # Check checksum
        checksum = 0
        for i in range(data_len + 1):
            checksum += response[i]
        if checksum & 0xFF != 0:
            print("Invalid checksum")
            return None
            
        # Return data
        return response[1:1+data_len-1]

def send_command(self, data):
        """Send a command frame to the PN532"""
        assert data is not None and 1 < len(data) < 255, 'Data must be array of 1 to 255 bytes.'
        
        # Build command frame
        cmd = bytearray(len(data) + 7)
        cmd[0] = 0x00  # Preamble
        cmd[1] = 0x00  # Start code
        cmd[2] = 0xFF  # Start code
        cmd[3] = len(data) + 1  # Length of data + TFI
        cmd[4] = (~cmd[3] + 1) & 0xFF  # Checksum of length
        cmd[5:5+len(data)] = data  # Data
        
        # Calculate checksum
        checksum = sum(data) & 0xFF
        cmd[5+len(data)] = ~checksum & 0xFF  # Checksum of data
        cmd[6+len(data)] = 0x00  # Postamble
        
        # Send command
        self.write_data(bytes(cmd))

    def write_data(self, data):
        """Write a command to the PN532"""
        self._uart.write(data)

    def read_data(self, count):
        """Read response data from the PN532"""
        frame = self._uart.read(count)
        if not frame:
            raise RuntimeError('Timeout waiting for response!')
        return bytearray(frame)

    def _wakeup(self):
        """Send any special commands/data to wake up PN532"""
        # Send frame with just the preamble
        preamble = bytearray(7)
        preamble[0] = 0x55
        preamble[1] = 0x55
        preamble[2] = 0x00
        preamble[3] = 0x00
        preamble[4] = 0x00
        preamble[5] = 0x00
        preamble[6] = 0x00
        self._uart.write(preamble)
        
        # Wait for serial data
        if not self._wait_ready(timeout=0.1):
            print("Warning: Wakeup timeout")
