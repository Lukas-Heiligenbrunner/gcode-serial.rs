pub enum SerialConnector {
    // automatically connect to first serial port found
    Auto,
    // manually specify (serialport, boudrate)
    Manual(String, u32),
}
