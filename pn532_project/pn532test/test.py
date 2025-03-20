import subprocess
import RPi.GPIO as GPIO

import pn532.pn532 as nfc
from pn532 import *

key_a = b'\xFF\xFF\xFF\xFF\xFF\xFF'


def _dump_test(dev):
    ic, ver, rev, support = dev.get_firmware_version()
    dev.SAM_configuration()
    print('请将卡贴近线圈')
    while True:
        uid = dev.read_passive_target(timeout=0.5)
        print('.', end="")
        if uid is not None:
            break
    print('卡 UID:', [hex(i) for i in uid])
    for i in range(4):
        try:
            dev.mifare_classic_authenticate_block(
                uid, block_number=i, key_number=nfc.MIFARE_CMD_AUTH_A, key=key_a)
            print(i, ':', ' '.join(['%02X' % x
                for x in dev.mifare_classic_read_block(i)]))
        except nfc.PN532Error as e:
            print(e.errmsg)
            break
    GPIO.cleanup()


def spi_test():
    print('''=== SPI 测试 ===
-   跳线帽连接 I0 <--> L
-   跳线帽连接 I1 <--> H
-   拨码开关开启SPI（11110000）''')
    input('回车继续...')
    try:
        dev = PN532_SPI(cs=4, reset=20, debug=False)
        _dump_test(dev)
        GPIO.cleanup()
        print('SPI 测试成功')
        return 0
    except Exception as e:
        print(e)
        print('SPI 初始化失败')
        GPIO.cleanup()
        return -1


def i2c_test():
    print('''=== I2C 测试 ===
-   跳线帽连接 I0 <--> H
-   跳线帽连接 I1 <--> L
-   拨码开关开启I2C（00001100）''')
    input('回车继续...')
    try:
        dev = PN532_I2C(req=16, reset=20, debug=False)
        _dump_test(dev)
        GPIO.cleanup()
        print('I2C 测试成功')
        return 0
    except Exception as e:
        print(e)
        print('I2C 初始化失败')
        GPIO.cleanup()
        return -1


def uart_test():
    print('''=== UART 测试 ===
-   跳线帽连接 I0 <--> L
-   跳线帽连接 I1 <--> L
-   拨码开关开启UART（00000011）''')
    input('回车继续...')
    try:
        dev = PN532_UART(reset=20, debug=False)
        _dump_test(dev)
        GPIO.cleanup()
        print('UART 测试成功')
        return 0
    except Exception as e:
        print(e)
        print('UART 初始化失败')
        GPIO.cleanup()
        return -1


def io_test():
    print('''=== IO 测试 ===
-   跳线帽连接 I0 <--> L
-   跳线帽连接 I1 <--> L
-   拨码开关开启UART（00000011）''')
    input('回车继续...')
    io = (1 << 2) | (1 << 4) & 0xFF
    try:
        dev = PN532_UART(reset=20, debug=False)
        dev.write_gpio(p3=io)
        for bit in [0, 1, 3, 5]:
            dev.write_gpio(p3=io | (1 << bit))
            p3, _, _ = dev.read_gpio()
            if p3 != (io | (1 << bit)):
                print('IO 测试失败。写入：%02x，读出：%02x' % (io | (1 << bit), p3))
                return -1
        GPIO.cleanup()
        print('IO 测试成功')
        return 0
    except Exception as e:
        print(e)
        print('UART 初始化失败')
        GPIO.cleanup()
        return -1


def format_mifare_classic():
    print('''=== 格式化卡 ===
-   跳线帽连接 I0 <--> L
-   跳线帽连接 I1 <--> L
-   拨码开关开启UART（00000011）
''')
    input('必须提前将卡贴近线圈，再按回车，才能正常格式化')
    pipe = subprocess.Popen('nfc-mfclassic f A u dummy.mfd dummy.mfd f',
            shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    err = pipe.stderr.read()
    if err:
        print(err.decode('utf-8'))
        return -1
    results = pipe.stdout.readlines()
    if results[-1].decode('utf-8').startswith('Error: no tag was found'):
        print('请预先将卡贴近线圈，再重试')
        return -1
    if results[-1].decode('utf-8').startswith('Done'):
        print('格式化成功')
    pipe = subprocess.Popen('nfc-mfsetuid',
            shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    err = pipe.stderr.read()
    if err:
        print(err.decode('utf-8'))
        return -1
    results = pipe.stdout.readlines()
    if not results[-1].decode('utf-8').startswith('Received'):
        print('设置UID失败，不是魔法卡或者卡密码错误')
        return -1
    print('设置UID成功')
    return 0


if __name__ == '__main__':
    GPIO.setwarnings(False)
    STEP_MAX = 5
    print('''=== PN532 NFC HAT 测试 ===
-   跳线帽连接 INT0 <--> D16
-   跳线帽连接 RSTPDN <--> D20
-   按下 Ctrl + C 退出
''')
    while True:
        step = input('''回车继续，或者输入编号，再按回车继续...
1.  SPI 测试
2.  I2C 测试
3.  UART 测试
4.  IO 测试
5.  格式化卡
''')
        try:
            step = 1 if not step else int(step)
        except ValueError:
            print('输入的必须是数字')
            continue
        if 0 < step <= STEP_MAX:
            break
    while step <= STEP_MAX:
        if step == 1:
            if spi_test():
                print('!!!!!!!!!! SPI 测试失败 !!!!!!!!!!')
                break
        elif step == 2:
            if i2c_test():
                print('!!!!!!!!!! I2C 测试失败 !!!!!!!!!!')
                break
        elif step == 3:
            if uart_test():
                print('!!!!!!!!!! UART 测试失败 !!!!!!!!!!')
                break
        elif step == 4:
            if io_test():
                print('!!!!!!!!!! IO 测试失败 !!!!!!!!!!')
                break
        elif step == 5:
            if format_mifare_classic():
                print('!!!!!!!!!! 格式化卡失败 !!!!!!!!!!')
                break
        step += 1
    print('测试结束')
    GPIO.cleanup()
