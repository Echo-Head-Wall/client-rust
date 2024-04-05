/*
 * Copyright 2023, Sayan Nandan <nandansayan@outlook.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
*/

//! # Configuration
//!
//! This module provides items to help with database connection setup and configuration.
//!
//! ## Example
//!
//! ```no_run
//! use skytable::Config;
//!
//! // establish a sync connection to 127.0.0.1:2003
//! let mut db = Config::new_default("username", "password").connect().unwrap();
//!
//! // establish a connection to a specific host `subnetx2_db1` and port `2008`
//! let mut db = Config::new("subnetx2_db1", 2008, "username", "password").connect().unwrap();
//! ```

use crate::protocol::handshake::ProtocolVersion;

/// The default host
///
/// NOTE: If you are using a clustering setup, don't use this!
pub const DEFAULT_HOST: &str = "127.0.0.1";
/// The default TCP port (skyhash/tcp)
pub const DEFAULT_TCP_PORT: u16 = 2003;
/// The default TLS port (skyhash/tls)
pub const DEFAULT_TLS_PORT: u16 = 2002;

#[derive(Debug, Clone, PartialEq)]
/// Configuration for a Skytable connection
pub struct Config {
    host: Box<str>,
    port: u16,
    username: Box<str>,
    password: Box<str>,
    pub(crate) protocol: ProtocolVersion,
}

impl Config {
    fn _new(
        host: Box<str>,
        port: u16,
        username: Box<str>,
        password: Box<str>,
        protocol: ProtocolVersion,
    ) -> Self {
        Self {
            host,
            port,
            username,
            password,
            protocol,
        }
    }
    /// Create a new [`Config`] using the default connection settings and using the provided username and password
    pub fn new_default(username: &str, password: &str) -> Self {
        Self::new(DEFAULT_HOST, DEFAULT_TCP_PORT, username, password)
    }
    /// Create a new [`Config`] using the given settings.
    ///
    /// **PROTOCOL VERSION**: Defaults to [`ProtocolVersion::V2_0`]
    pub fn new(host: &str, port: u16, username: &str, password: &str) -> Self {
        Self::_new(
            host.into(),
            port,
            username.into(),
            password.into(),
            ProtocolVersion::V2_0,
        )
    }
    /// Returns the host setting for this this configuration
    pub fn host(&self) -> &str {
        self.host.as_ref()
    }
    /// Returns the port setting for this this configuration
    pub fn port(&self) -> u16 {
        self.port
    }
    /// Returns the username setting for this this configuration
    pub fn username(&self) -> &str {
        self.username.as_ref()
    }
    /// Returns the password set using this configuration
    pub fn password(&self) -> &str {
        self.password.as_ref()
    }
}
