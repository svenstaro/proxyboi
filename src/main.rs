use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "proxyboi",
    raw(global_settings = "&[structopt::clap::AppSettings::ColoredHelp]")
)]
struct Args {
    #[strucopt(short, long)]

}

fn main() {
    println!("Hello, world!");
}
