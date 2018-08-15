# PJLink

This is a Rust library for the PJLink protocol.  PJLink is a network control protocol that has been incorporated into projectors and displays over the last few years.  You can find the protocol specification [here](https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=1&cad=rja&uact=8&ved=2ahUKEwj6s-zOkODcAhWEG3wKHbagAloQFjAAegQIABAC&url=https%3A%2F%2Fpjlink.jbmia.or.jp%2Fenglish%2Fdata%2F5-1_PJLink_eng_20131210.pdf&usg=AOvVaw3eWuyry5fcVR1_R-jxrK7J). This Library currently supports both authenticated and open connections and currently returns unparsed response.  This is just beginning of the API and more will be coming. However, I do plan to leave the raw send_command function for those that want to use this library at a lower level.

Testing has been done with Panasonic and Sanyo projectors.

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]

pjlink = "0.1.0"
```

Create a PjlinkDevice and start sending DMX data to a universe.

```rust
extern crate pjlink;
use pjlink::PjlinkDevice;

let mut device = PjlinkDevice::new("192.168.1.1").unwrap();

match device.send_command("POWR ?") {
    Ok(response) => println!("{}", response),
    Err(err) => println!("An error occurred: {}", err),
}

```

## License

Licensed under

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)


### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be licensed as above, without any additional terms or
conditions.
