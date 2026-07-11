#include <Arduino.h>
#include <Wire.h>
#include "DmxInput.h"

DmxInput dmxInput;

#define START_CHANNEL 0   // include start byte
#define NUM_CHANNELS 513  // 513
#define BUFFER_SIZE DMXINPUT_BUFFER_SIZE(START_CHANNEL, NUM_CHANNELS)
#define DMX_RX_PIN 2
#define DMX_EN_PIN 3
#define DMX_TX_PIN 4

// Buffer for use with DMX library
volatile uint8_t buffer[BUFFER_SIZE];

// Buffer to share data between cores
volatile uint8_t sharedData[BUFFER_SIZE];

// Flag to signal new data is available (needs synchronization)
volatile bool newDataReady = false;

void setup() {
  Serial.begin(115200);

  // Setup our DMX Input to read on GPIO 2
  dmxInput.begin(DMX_RX_PIN, START_CHANNEL, NUM_CHANNELS);
  dmxInput.read_async(buffer);

  // Setup the onboard LED so that we can blink when we receives packets
  pinMode(LED_BUILTIN, OUTPUT);

  pinMode(DMX_EN_PIN, OUTPUT);
  digitalWrite(DMX_EN_PIN, LOW);

  rp2040.fifo.begin(2);
}

void loop() {
  delay(30);

  if (millis() > 100 + dmxInput.latest_packet_timestamp()) {
    Serial.println("no data!");
    return;
  }

  if (!newDataReady) {  // Only fill if Core 1 has processed previous data
    for (int i = 0; i < BUFFER_SIZE; i++) {
      // sharedData[i] = i % 256;
      sharedData[i] = buffer[i];
    }
    Serial.println("Core 0: Filled data buffer.");
    newDataReady = true;  // Signal new data is ready for Core 1
    rp2040.fifo.push(1);  // Push a dummy value to FIFO to wake up Core 1
  }

  // Blink the LED to indicate that a packet was received
  digitalWrite(LED_BUILTIN, HIGH);
  delay(10);
  digitalWrite(LED_BUILTIN, LOW);
}

// byte core1Data[DMXINPUT_BUFFER_SIZE(START_CHANNEL, NUM_CHANNELS)];
uint8_t core1Data[BUFFER_SIZE];
volatile bool i2cDataRequest = false;
volatile uint8_t i2cCommand = 0x00;

void setup1() {
  //   Serial.begin(115200);

  Wire1.setSDA(6);  //GPIO6 = pin 9
  Wire1.setSCL(7);  //GPIO7 = pin 10
  Wire1.setClock(400000);  // 400kHz fast mode (was 4000000, not a valid I2C clock; ignored in slave mode anyway)
  Wire1.begin(0x33);
  Wire1.onReceive(receiveEvent);
  Wire1.onRequest(req);
}


void loop1() {

  if (rp2040.fifo.available()) {  // Check if FIFO has data (signal from Core 0)
    rp2040.fifo.pop();            // Pop the dummy value
    // Serial.println("Core 1: Received signal from Core 0.");

    // Process the data in the shared buffer
    for (int i = 0; i < BUFFER_SIZE; i++) {
      core1Data[i] = sharedData[i];
    }

    newDataReady = false;  // Reset the flag, allowing Core 0 to fill again
                           // Serial.println("Core 1: Data transferred.");
  }


  if (i2cDataRequest == true) {
    // Serial.println("Sending I2C data");
    switch (i2cCommand) {
      case 0x01:
        Wire1.write(&core1Data[0], 200);  // send 200 bytes starting at 0 [0-199]
        i2cDataRequest = false;
        break;
      case 0x02:
        Wire1.write(&core1Data[200], 200);  // send 200 bytes starting at 200 [200-399]
        i2cDataRequest = false;
        break;
      case 0x03:
        Wire1.write(&core1Data[400], 113);  // send 113 bytes starting at 400 [400-513]
        i2cDataRequest = false;
        break;
    }
  }
}

// These are called in an **INTERRUPT CONTEXT** which means NO serial port
// access (i.e. Serial.print is illegal) and no memory allocations, etc.

// Called when the I2C slave is read from
void req() {
  i2cDataRequest = true;
}

// Called when the I2C slave is read from
void receiveEvent(int numBytes) {
  while (Wire1.available()) {  // Read Any Received Data
    i2cCommand = Wire1.read();
  }
}