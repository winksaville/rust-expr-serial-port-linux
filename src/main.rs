// Initial code from:
//   https://gitlab.com/susurrus/serialport-rs/-/blob/master/examples/clear_input_buffer.rs

//use std::{array::FixedSizeArray, error::Error};
use std::error::Error;
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::{App, AppSettings, Arg};

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

    let mut port = serialport::new(port_name, rate)
        .timeout(Duration::from_millis(10))
        .open()
        .map_err(|ref e| format!("Port '{}' not available: {}", &port_name, e))?;

    let chan_user_buf = input_service();

    println!("Connected to {} at {} baud", &port_name, &baud_rate);
    println!("Ctrl+D (Unix) or Ctrl+Z (Win) to stop. Press Return to clear the buffer.");

    loop {
        match chan_user_buf.try_recv() {
            Ok(buf) => {
                // let s = format!("{}\r", buf);
                // println!("++++ s.len={} ++++", s.len());
                // println!("{:?}", s);
                // let b = s.as_bytes();
                // println!("{:?}", b);
                // //port.write_all(b).unwrap();
                // let size_written = match port.write(b) {
                //     Ok(s) => s,
                //     Err(e) => panic!("Error port.write: {}", e)
                // };
                // println!("+++++++++++++++ size_written: {} ", size_written);
                println!("++++ buf.len={} ++++", buf.len());
                println!("buf: {:?}", buf);
                let b = buf.as_bytes();
                println!("b: {:?}", b);
                port.write_all(b).unwrap();

                // Write trailing CR
                println!("write CR");
                let mut v: Vec<u8> = Vec::new();
                v.push(13);
                let size_written = match port.write(&v) {
                    Ok(s) => s,
                    Err(e) => panic!("Error port.write: {}", e)
                };
                println!("+++++++++++++++ size_written: {} ", size_written);
            }
            Err(mpsc::TryRecvError::Empty) => (),
            Err(mpsc::TryRecvError::Disconnected) => {
                println!("Stopping.");
                break;
            }
        }

        // This panics with an "IO error" when I "Write trailing CR"
        let read_count = match port.bytes_to_read() {
            Ok(c) => c,
            Err(e) => {
                println!("Error bytes_to_read: {}", e);
                0
            }
        };

        if read_count != 0 {
            println!("Bytes available to read: {}", read_count);

            let mut serial_buf: Vec<u8> = vec![0; 1000];
            match port.read(serial_buf.as_mut_slice()) {
                Ok(size) => {
                    let s = match std::str::from_utf8(&serial_buf[..size]) {
                        Ok(v) => v,
                        Err(e) => panic!("Invalid utf8: {}", e),
                    };
                    println!("---- port size={} {:?} ----", size, s);
                    //io::stdout().write_all(&serial_buf[..size]).unwrap();
                    //io::stdout().write(s);
                    println!("{}", s);
                    println!("--------------");
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn input_service() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();

    thread::spawn(move || {
        loop {
            // Block awaiting any user input
            let mut buffer = String::new();
            println!("it: readline");
            match io::stdin().read_line(&mut buffer) {
                Ok(0) => {
                    println!("it: drop(tx)");
                    drop(tx); // EOF, drop the channel and stop the thread
                    break;
                }
                Ok(size) => {
                    // Remove "\n" at end of string and send result
                    let bytes = buffer[..size-1].to_owned();
                    println!("it: send {:?}", bytes);
                    tx.send(bytes).unwrap();
                }
                Err(e) => {
                    println!("it: panic");
                    panic!(e);
                }
            }
        }
        println!("it: done");
    });

    println!("input_service: return rx");
    rx
}

