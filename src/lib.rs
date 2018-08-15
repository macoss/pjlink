// Copyright 2018 Rick Russell
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::prelude::*;
use std::net::TcpStream;
use std::io::{Error,ErrorKind};

extern crate md5;

const AUTH: char = '1';
const NOAUTH: char = '0';
static PORT: &'static str = "4352";


pub struct PjlinkDevice {
    
    host: String,
    password: String,
    managed: bool, // Currently not implemented but will add monitoring support with call backs with the status changes
}

impl PjlinkDevice {
    /// Constructs a new PjlinkDevice.
    pub fn new(host: &str) -> Result<PjlinkDevice,Box<Error>> {
        let pwd = String::from("");
        PjlinkDevice::new_with_password(host, &pwd)
    }

    /// Contructs a new PjlinkDevice that has a password
    pub fn new_with_password(host: &str, password: &str) -> Result<PjlinkDevice,Box<Error>> {
        Ok(PjlinkDevice {
            host: host.to_string(),
            password: String::from(password),
            managed: false, // Hard coded to start until implemented.
        })
    }

    /// Send a command to the device and returs and error is there is a problem
    pub fn send_command(&self, command: &str) -> Result<String, Box<Error>> {
        let host_port = [&self.host, ":", PORT].concat();
        let mut client_buffer = [0u8; 256];
        let mut stream = try!(TcpStream::connect(host_port));

        let _ = stream.read(&mut client_buffer); //Did we get the hello string?

        let cmd: String = match client_buffer[7] as char { // Does the connection require auth or not
            AUTH => { // Connection requires auth
                let rnd_num = String::from_utf8_lossy(&client_buffer[9..17]).to_string();
                let pwd_str = format!("{}{}", rnd_num, &self.password);
                let digest = md5::compute(pwd_str);
                format!("{:x}%1{}\r", digest, command)
            },
            NOAUTH => { // Connection requires no auth
                format!("%1{}\r", command)
            },

            _ => return Err(Box::new(Error::new(ErrorKind::InvalidInput, "Invalid response or is not a PJLink device")))
        };
        
        let result = stream.write(cmd.as_bytes());
        match result {
            Ok(_) => (),
            Err(e) => return Err(Box::new(e)),
        };
        let result = stream.read(&mut client_buffer);
        let len = match result {
            Ok(len) => len,
            Err(e) => return Err(Box::new(e)), 
        };

        let response = String::from_utf8_lossy(&client_buffer[0..len]).to_string();
        Ok(response)
    }
}
