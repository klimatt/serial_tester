use std::{env, io, str};
use tokio_util::codec::{Encoder, Decoder};
use futures::{sink::SinkExt, stream::StreamExt};
use bytes::{BytesMut, BufMut};
use rand::prelude::*;
use tokio::time::{timeout, Duration};
use tokio_serial::SerialPort;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/tty.usbserial-A50285BI";
const DEFAULT_BAUD: &str = "9600";

struct SerialTester;

impl Decoder for SerialTester {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, acc: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !acc.is_empty() {
            let res = acc.to_vec();
            acc.clear();
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for SerialTester {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        buf.reserve(item.len());
        buf.put_slice(item.as_slice());
        Ok(())
    }
}



#[tokio::main]
async fn main() {
    let mut args = env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());
    //let baud = args.nth(2).unwrap_or_else(|| DEFAULT_BAUD.into()).parse::<u32>().unwrap();


    let mut settings = tokio_serial::SerialPortSettings::default();
    let mut port = tokio_serial::Serial::from_path(tty_path.clone(), &settings).unwrap();

    let mut random_array = vec![0u8; 256];
    rand::thread_rng().fill_bytes(&mut random_array);

    let baud : u32 = 110;

    println!("======================");
    println!("Start sending {:?} bytes on {:?} baud to {:?}....",random_array.len() , baud, tty_path);

    port.set_baud_rate(baud);

    #[cfg(unix)]
        port.set_exclusive(false)
        .expect("Unable to set serial port exclusive to false");

    let (mut tx, mut rx) = SerialTester.framed(port).split();

    tx.send(random_array.clone()).await;

    let mut it_r = random_array.iter();

    let mut cnt: u32 = 0;

    let mut err_cnt: usize = 0;

    let res = timeout(Duration::from_secs(4), async {
        while let Some(rx_bytes) = rx.next().await {
            let byte_array = rx_bytes.unwrap();
            let mut it_b = byte_array.iter();

            while let Some(r_b) = it_b.next()
            {
                if let Some(g_b) = it_r.next() {
                    if r_b != g_b
                    {
                        err_cnt += 1;
                        println!("[{:?}] :: {:?} != {:?}", cnt, r_b, g_b);
                    }
                    cnt += 1;
                }
            }
        }
    }).await;

    if res.is_err() {
        let err_percent: f32 = (err_cnt as f32 * 100.0) / random_array.len() as f32;
        println!("RES :: Recv_len = {:?}; Err_cnt = {:?}; Err_% = {:.3}", cnt, err_cnt, err_percent);
    }
    println!("======================");
}

