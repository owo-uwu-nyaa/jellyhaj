use color_eyre::Result;

fn main() -> Result<()> {
    let res = config::effects::parse_colors()?;
    println!("colors: {res:?}");
    Ok(())
}
