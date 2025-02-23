/*----------------------------------------------------------------------------------------------------------
 *  Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/conclave-room-net-rs
 *  Licensed under the MIT License. See LICENSE in the project root for license information.
 *--------------------------------------------------------------------------------------------------------*/
//! The Conclave Net Layer
//!
//! Easier to handle incoming network commands and construct outgoing messages
use std::time::Instant;

use conclave_room::{ConnectionIndex, Room};
use conclave_room_serialize::{RoomInfoCommand, ServerReceiveCommand};
use flood_rs::{OutOctetStream, ReadOctetStream};

pub struct NetworkConnection {
    pub id: ConnectionIndex,
    pub room: Room,
}

pub trait SendDatagram {
    fn send(&self) -> Vec<u8>;
}

impl SendDatagram for Room {
    fn send(&self) -> Vec<u8> {
        let room_info_command = RoomInfoCommand {
            term: self.term,
            leader_index: self.leader_index,
            client_infos: vec![],
        };

        let mut stream = OutOctetStream::new();

        room_info_command
            .to_octets(&mut stream)
            .expect("Failed to write command {room_info_command:?} to octet stream");

        stream.data
    }
}

pub trait ReceiveDatagram {
    fn receive(
        &mut self,
        connection_id: ConnectionIndex,
        now: Instant,
        buffer: &mut impl ReadOctetStream,
    ) -> Result<(), String>;
}

impl ReceiveDatagram for Room {
    fn receive(
        &mut self,
        connection_id: ConnectionIndex,
        now: Instant,
        reader: &mut impl ReadOctetStream,
    ) -> Result<(), String> {
        if !self.connections.contains_key(&connection_id) {
            return Err(format!("there is no connection {}", connection_id));
        }
        let command = ServerReceiveCommand::from_cursor(reader).unwrap();
        match command {
            ServerReceiveCommand::PingCommandType(ping_command) => {
                self.on_ping(
                    connection_id,
                    ping_command.term,
                    ping_command.has_connection_to_leader,
                    ping_command.knowledge,
                    now,
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use conclave_room::Room;
    use conclave_room_serialize::PING_COMMAND_TYPE_ID;
    use flood_rs::InOctetStream;

    use crate::{ReceiveDatagram, SendDatagram};

    #[test]
    fn check_send() {
        let room = Room::new();
        let octets = room.send();

        assert_eq!(vec![0x00, 0x00, 0x00, 0xff], octets);
    }

    #[test]
    fn on_ping() {
        const EXPECTED_KNOWLEDGE_VALUE: u64 = 17718865395771014920;
        let octets = [
            PING_COMMAND_TYPE_ID,
            0x00, // Term
            0x20,
            0xF5, // Knowledge
            0xE6,
            0x0E,
            0x32,
            0xE9,
            0xE4,
            0x7F,
            0x08,
            0x01, // Has connection to leader
        ];
        let mut receive_cursor = InOctetStream::new(octets.into());

        let mut room = Room::new();
        let now = Instant::now();
        let first_connection_id = room.create_connection(now);
        let receive_result = room.receive(first_connection_id, now, &mut receive_cursor);
        assert_eq!(receive_result, Ok(()));

        let connection_after_receive = room.connections.get(&first_connection_id).unwrap();
        assert_eq!(connection_after_receive.knowledge, EXPECTED_KNOWLEDGE_VALUE);
    }
}
