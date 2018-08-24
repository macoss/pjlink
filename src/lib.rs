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

// Return the correct error message based on the PJ Link specification
fn pjlink_error(error_msg: &str) -> Error {
    match &error_msg[0..4] {
        "ERR1" => Error::new(ErrorKind::InvalidData, format!("Undefined command")),
        "ERR2" => Error::new(ErrorKind::InvalidData, format!("Invalid parameter")),
        "ERR3" => Error::new(ErrorKind::InvalidData, format!("Unavaiable at this time")),
        "ERR4" => Error::new(ErrorKind::InvalidData, format!("Projector/Display Failure")),
        "ERRA" => Error::new(ErrorKind::PermissionDenied, format!("Authorization Error")),
        _ => Error::new(ErrorKind::InvalidData, format!("Error reported from the projector {}", error_msg)),
    }
}

// Parse the response from the device
fn parse_response(response: &str) -> Result<PjlinkResponse, Error> {
    let mut equals_sign: usize = 0;
    let len = response.len();
    //lets find the equals sign
    for (i, c) in response.chars().enumerate() {
        if c == '=' || c == ' ' {
            equals_sign = i;
            break;
        }
    }
    
    let command = if &response[0..1] != "%" {
        CommandType::PJLINK
    } else {
        match &response[2..equals_sign] {
            "POWR" => CommandType::Power,
            "INPT" => CommandType::Input,
            "AVMT" => CommandType::AvMute,
            "ERST" => CommandType::ErrorStatus,
            "LAMP" => CommandType::Lamp,
            "INST" => CommandType::InputList,
            "NAME" => CommandType::Name,
            "INF1" => CommandType::Manufacturer,
            "INF2" => CommandType::ProductName,
            "INFO" => CommandType::Information,
            "CLSS" => CommandType::Class,
            _ => return Err(Error::new(ErrorKind::InvalidInput, "Invalid command type returned.")),
        }
    };

    let value = &response[equals_sign+1..len];

    // Did we get and error report and if so lets return it so the functions don't have check for errors.
    if len == 3 {
        if &value[0..3] == "ERR" {
            return Err(pjlink_error(value));
        }
    }

    Ok(
        PjlinkResponse{
            action: command,
            value: value.to_string(),
        }
    )

}

// This is the list of standard command/response types from the PJLink spec. 
// At this point I would think that this would only be used internally.
enum CommandType {
    PJLINK,
    Power,
    Input,
    AvMute,
    ErrorStatus,
    Lamp,
    InputList,
    Name,
    Manufacturer,
    ProductName,
    Information,
    Class,
}

/// Power status definitions
pub enum PowerStatus {
    Off,
    On,
    Cooling,
    Warmup,
}

pub enum InputType {
    RGB(u8),
    Video(u8),
    Digital(u8),
    Storage(u8),
    Network(u8),
}

struct PjlinkResponse {
    action: CommandType,
    value: String,
}

pub struct PjlinkDevice {
    host: String,
    password: String,
    managed: bool, // Currently not implemented but will add monitoring support with call backs with the status changes
}

impl PjlinkDevice {
    /// Constructs a new PjlinkDevice.
    pub fn new(host: &str) -> Result<PjlinkDevice, Error> {
        let pwd = String::from("");
        PjlinkDevice::new_with_password(host, &pwd)
    }

    /// Contructs a new PjlinkDevice that has a password
    pub fn new_with_password(host: &str, password: &str) -> Result<PjlinkDevice, Error> {
        Ok(PjlinkDevice {
            host: host.to_string(),
            password: String::from(password),
            managed: false, // Hard coded to start until implemented.
        })
    }

    /// Send a command and a Result with the raw string or an error
    pub fn send_command(&self, command: &str) -> Result<String, Error> {
        let host_port = [&self.host, ":", PORT].concat();
        let mut client_buffer = [0u8; 256];
        let mut stream = try!(TcpStream::connect(host_port));

        let _ = stream.read(&mut client_buffer); //Did we get the hello string?
        
        let cmd: String = match client_buffer[7] as char { // Does the connection require auth or not
            AUTH => { // Connection requires auth
                let rnd_num = String::from_utf8_lossy(&client_buffer[9..17]).to_string();
                if &self.password != "" { // We got a password
                    let pwd_str = format!("{}{}", rnd_num, &self.password);
                    let digest = md5::compute(pwd_str);
                    format!("{:x}%1{}\r", digest, command)
                } else { // No password was supplied so we are going to raise an error.
                    return Err(Error::new(ErrorKind::InvalidInput, "This device requires a password and one was not supplied."))
                }
            },
            NOAUTH => { // Connection requires no auth
                format!("%1{}\r", command)
            },

            _ => return Err(Error::new(ErrorKind::InvalidInput, "Invalid response or is not a PJLink device"))
        };
        
        let result = stream.write(cmd.as_bytes());
        match result {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        let result = stream.read(&mut client_buffer);
        let len = match result {
            Ok(len) => len,
            Err(e) => return Err(e), 
        };

        let response = String::from_utf8_lossy(&client_buffer[0..len-1]).to_string();   
        Ok(response)
    }

    // a wrapper around send_command that will parse the response
    fn send(&self, cmd: &str) -> Result<PjlinkResponse, Error> {
        match self.send_command(cmd) {
            Ok(send_result) => {
                match parse_response(&send_result) {
                    Ok(parse_result) => Ok(parse_result),
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(e),
        }
    }

    /// Check the power status of the device and returns an enum 
    pub fn get_power_status(&self) -> Result<PowerStatus, Error> {
        match self.send("POWR ?") {
            Ok(result) => {
                match result.action {
                    CommandType::Power => {
                        match &result.value[0..1] {
                            "0" => Ok(PowerStatus::Off),
                            "1" => Ok(PowerStatus::On),
                            "2" => Ok(PowerStatus::Cooling),
                            "3" => Ok(PowerStatus::Warmup),
                            _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response: {}", result.value))), // Invalid Response
                        }
                    } 
                _ => Err(Error::new(ErrorKind::InvalidInput, format!("Got a response we didn't expect: {}", result.value)))
                }
            },
            Err(e) => Err(e),
        }
    }

    /// Turn on the device
    pub fn power_on(&self) -> Result<PowerStatus, Error> {
        match self.send("POWR 1") {
            Ok(result) => {
                match result.action {
                    CommandType::Power => {
                        match &result.value[0..2] {
                            "OK" => {
                                match self.get_power_status() {
                                    Ok(status) => Ok(status),
                                    Err(e) => Err(e),
                                }
                            },
                            _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response: {}", result.value))), // Invalid Response
                        }
                    },
                    _ =>Err(Error::new(ErrorKind::InvalidInput, format!("Got a response we didn't expect: {}", result.value)))
                }
            },
            Err(e) => Err(e),            
        }
    }

    /// Turn off the device
    pub fn power_off(&self) -> Result<PowerStatus, Error> {
        match self.send("POWR 0") {
            Ok(result) => {
                match result.action {
                    CommandType::Power => {
                        match &result.value[0..2] {
                            "OK" => {
                                match self.get_power_status() {
                                    Ok(status) => Ok(status),
                                    Err(e) => Err(e),
                                }
                            },
                            _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response: {}", result.value))), // Invalid Response
                        }
                    
                    },   
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Got a response we didn't expect: {}", result.value)))
                }
            },
            Err(e) => Err(e),            
        }
    }

    /// Get the information (INFO ?) from the 
    pub fn get_info(&self) -> Result<String, Error> {
        match self.send("INFO ?") {
            Ok(result) => {
                match result.action {
                    CommandType::Information => {
                        Ok(result.value)
                    },
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response:: {}", result.value))),
                }
            },
            Err(e) => Err(e),  
        }
    }

    /// Get the manufacturer (INF1 ?) from the device
    pub fn get_manufacturer(&self) -> Result<String, Error> {
        match self.send("INF1 ?") {
            Ok(result) => {
                match result.action {
                    CommandType::Manufacturer => {
                        Ok(result.value)
                    },
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response:: {}", result.value))),
                }
            },
            Err(e) => Err(e),  
        }
    }

    /// Get the product name (INF2 ?) from the device
    pub fn get_product_name(&self) -> Result<String, Error> {
        match self.send("INF2 ?") {
            Ok(result) => {
                match result.action {
                    CommandType::ProductName => {
                        Ok(result.value)
                    },
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response:: {}", result.value))),
                }
            },
            Err(e) => Err(e),  
        }
    }
    /// Get the product class (CLSS ?) from the device
    pub fn get_class(&self) -> Result<String, Error> {
        match self.send("CLSS ?") {
            Ok(result) => {
                match result.action {
                    CommandType::Class => {
                        Ok(result.value)
                    },
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response:: {}", result.value))),
                }
            },
            Err(e) => Err(e),  
        }
    }
    
    /// Get the device name (NAME ?) from the device
    pub fn get_device_name(&self) -> Result<String, Error> {
        match self.send("NAME ?") {
            Ok(result) => {
                match result.action {
                    CommandType::Name => {
                        Ok(result.value)
                    },
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response:: {}", result.value))),
                }
            },
            Err(e) => Err(e),  
        }
    }

    /// Get the current input (INPT ?) from the device
    pub fn get_input(&self) -> Result<InputType, Error> {
        match self.send("INPT ?") {
            Ok(result) => {
                let input = result.value.parse::<u8>().unwrap();
                match input {
                    11...19 => Ok(InputType::RGB(input - 10)),
                    21...29 => Ok(InputType::Video(input - 20)),
                    31...39 => Ok(InputType::Digital(input - 30)),
                    41...49 => Ok(InputType::Storage(input - 40)),
                    51...59 => Ok(InputType::Network(input - 50)),
                    _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid input:: {}", input))),
                }
            },
            Err(e) => Err(e),
        }
    }
}
