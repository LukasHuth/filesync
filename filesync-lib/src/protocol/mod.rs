pub trait ServerSidePacketUtils {
    fn read(&mut self) -> impl Future<Output = Option<ServerBoundPacket>>;
    fn write(
        &mut self,
        packet: ClientBoundPacket,
    ) -> impl Future<Output = Result<(), Box<dyn std::error::Error>>>;
}
pub trait ClientSidePacketUtils {
    fn read(&mut self) -> impl Future<Output = Option<ClientBoundPacket>>;
    fn write(
        &mut self,
        packet: ServerBoundPacket,
    ) -> impl Future<Output = Result<(), Box<dyn std::error::Error>>>;
}
pub enum ServerBoundPacket {
    Authenticate {
    }
}
pub enum ClientBoundPacket {}
