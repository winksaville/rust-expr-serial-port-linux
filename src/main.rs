// Initial code from:
//   https://gitlab.com/susurrus/serialport-rs/-/blob/master/examples/clear_input_buffer.rs

use std::error::Error;
use std::io::{self, Read};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::{App, AppSettings, Arg};

use serialport::ClearBuffer;

fn main() {
    let matches = App::new("Serialport Example - Clear Input Buffer")
        .about("Reports how many bytes are waiting to be read and allows the user to clear the input buffer")
        .setting(AppSettings::DisableVersion)
        .arg(Arg::with_name("port")
             .help("The device path to a serial port")
             .use_delimiter(false)
             .required(true))
        .arg(Arg::with_name("baud")
             .help("The baud rate to connect at")
             .use_delimiter(false)
             .required(true))
        .get_matches();
    let port_name = matches.value_of("port").unwrap();
    let baud_rate = matches.value_of("baud").unwrap();

    let exit_code = match run(&port_name, &baud_rate) {
        Ok(_) => 0,
        Err(e) => {
            println!("Error: {}", e);
            1
        }
    };

    std::process::exit(exit_code);
}

fn run(port_name: &str, baud_rate: &str) -> Result<(), Box<dyn Error>> {
    let rate = baud_rate
        .parse::<u32>()
        .map_err(|_| format!("Invalid baud rate '{}' specified", baud_rate))?;

    let port = serialport::new(port_name, rate)
        .timeout(Duration::from_millis(10))
        .open()
        .map_err(|ref e| format!("Port '{}' not available: {}", &port_name, e))?;

    let chan_clear_buf = input_service();

    println!("Connected to {} at {} baud", &port_name, &baud_rate);
    println!("Ctrl+D (Unix) or Ctrl+Z (Win) to stop. Press Return to clear the buffer.");

    loop {
        println!(
            "Bytes available to read: {}",
            port.bytes_to_read().expect("Error calling bytes_to_read")
        );

        match chan_clear_buf.try_recv() {
            Ok(_) => {
                println!("------------------------- Discarding buffer ------------------------- ");
                port.clear(ClearBuffer::Input)
                    .expect("Failed to discard input buffer")
            }
            Err(mpsc::TryRecvError::Empty) => (),
            Err(mpsc::TryRecvError::Disconnected) => {
                println!("Stopping.");
                break;
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn input_service() -> mpsc::Receiver<()> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut buffer = [0; 32];
        loop {
            // Block awaiting any user input
            match io::stdin().read(&mut buffer) {
                Ok(0) => {
                    drop(tx); // EOF, drop the channel and stop the thread
                    break;
                }
                Ok(_) => tx.send(()).unwrap(), // Signal main to clear the buffer
                Err(e) => panic!(e),
            }
        }
    });

    rx
}

