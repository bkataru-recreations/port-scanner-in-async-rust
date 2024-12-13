use clap::Parser;
use std::net::IpAddr;

use tokio::{
    net::TcpStream,
    runtime::Runtime,
    sync::mpsc::{self},
};

#[derive(Debug, Parser)]
struct Args {
    /// IP address of the port scan.
    #[arg(conflicts_with("cidr"), required_unless_present("cidr"))]
    addr: Option<IpAddr>,

    #[arg(long)]
    cidr: Option<cidr::IpCidr>,

    /// Start of the range.
    #[arg(short = 's', long, default_value_t = 1)]
    port_start: u16,

    // End of the range of ports to scan (inclusive).
    #[arg(short = 'e', long, default_value_t = 1024)]
    port_end: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    assert!(args.port_start != 0);
    assert!(args.port_start <= args.port_end);

    let rt = Runtime::new()?;
    let (tx, mut rx) = mpsc::channel(10);

    // let mut open_ports = vec![];
    rt.block_on(async {
        let (mut from_single, mut from_cidr);

        let addresses: &mut dyn Iterator<Item = IpAddr>;

        match (args.addr, args.cidr) {
            (Some(addr), _) => {
                from_single = vec![addr].into_iter();
                addresses = &mut from_single;
            }
            (_, Some(cidr)) => {
                from_cidr = cidr.iter().map(|net| net.address());
                addresses = &mut from_cidr;
            }
            (_, _) => unreachable!(),
        }

        for addr in addresses {
            println!("? {addr}:{}-{}", args.port_start, args.port_end);
            for port in args.port_start..=args.port_end {
                let tx = tx.clone();
                tokio::spawn(async move {
                    if let Err(err) = scan(addr, port, tx).await {
                        eprintln!("error: {err}");
                    };
                });
            }
        }
    });

    drop(tx);

    while let Ok((addr, port)) = rx.try_recv() {
        println!("= {addr}:{port}");
    }

    Ok(())
}

async fn scan(
    addr: IpAddr,
    port: u16,
    results_tx: mpsc::Sender<(IpAddr, u16)>,
) -> Result<(), mpsc::error::SendError<(IpAddr, u16)>> {
    if let Ok(_ping) = TcpStream::connect((addr, port)).await {
        results_tx.send((addr, port)).await?;
        //&mut open_ports.push((args.addr, port));
    }

    Ok(())
}
