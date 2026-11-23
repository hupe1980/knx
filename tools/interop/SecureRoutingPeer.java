import io.calimero.DataUnitBuilder;
import io.calimero.SerialNumber;
import io.calimero.cemi.CEMILData;
import io.calimero.knxnetip.SecureConnection;
import io.calimero.knxnetip.servicetype.KNXnetIPHeader;
import io.calimero.knxnetip.servicetype.RoutingIndication;
import java.net.*;
import java.nio.file.*;
import java.util.HexFormat;
import java.time.Duration;
import java.util.concurrent.*;
import javax.crypto.spec.SecretKeySpec;

// Independent peer with a synthetic, pre-synchronized group timer on loopback.
class SecureRoutingPeer {
    public static void main(String[] args) throws Exception {
        var loopback = NetworkInterface.getByName(args[0]);
        if (args.length == 4 && args[3].equals("timer")) {
            var received = new CountDownLatch(1);
            var key = HexFormat.of().parseHex("000102030405060708090a0b0c0d0e0f");
            try (var routing = SecureConnection.newRouting(loopback, InetAddress.getByName(args[2]), key, Duration.ofMillis(100))) {
                routing.addConnectionListener(e -> {
                    if (e.getFrame() instanceof CEMILData f && f.getSource().getRawAddress() == 0x7102
                            && f.getDestination().getRawAddress() == 0x1c05 && DataUnitBuilder.getAPDUService(f.getPayload()) == 0) {
                        received.countDown();
                    }
                });
                Files.createFile(Path.of(args[1]));
                if (!received.await(7, TimeUnit.SECONDS)) throw new AssertionError("missing synchronized Rust group read");
            }
            System.out.println("Calimero synchronized the Rust timer and authenticated the subsequent multicast read.");
            return;
        }
        var target = new InetSocketAddress(args[2], 3671);
        var key = new SecretKeySpec(HexFormat.of().parseHex("000102030405060708090a0b0c0d0e0f"), "AES");
        try (var socket = new MulticastSocket(null)) {
            socket.setReuseAddress(true);
            socket.bind(target);
            socket.setNetworkInterface(loopback);
            socket.joinGroup(target, loopback);
            socket.setSoTimeout(6000);
            Files.createFile(Path.of(args[1]));
            var packet = new DatagramPacket(new byte[512], 512);
            socket.receive(packet);
            if (!packet.getAddress().isLoopbackAddress()) throw new AssertionError("foreign ingress");
            byte[] wire = packet.getData();
            var h = new KNXnetIPHeader(wire, 0);
            var fields = SecureConnection.unwrap(h, wire, h.getStructLength(), key);
            if (!fields[0].equals(0) || !fields[1].equals(1001L)
                    || !fields[2].equals(SerialNumber.of(0x010203040506L)) || !fields[3].equals(0))
                throw new AssertionError("unexpected secure routing header");
            byte[] inner = (byte[]) fields[4];
            var ih = new KNXnetIPHeader(inner, 0);
            if (ih.getServiceType() != KNXnetIPHeader.ROUTING_IND) throw new AssertionError("service");
            var routed = new RoutingIndication(inner, ih.getStructLength(), inner.length - ih.getStructLength());
            var frame = (CEMILData) routed.getCEMI();
            if (frame.getSource().getRawAddress() != 0x7102 || frame.getDestination().getRawAddress() != 0x1c05
                    || DataUnitBuilder.getAPDUService(frame.getPayload()) != 0)
                throw new AssertionError("unexpected Rust group read");
            var busy = HexFormat.of().parseHex("06100532000c060000640000");
            byte[] reply = SecureConnection.newSecurePacket(0, 1002, SerialNumber.of(7), 1, busy, key);
            try (var outbound = new MulticastSocket(null)) {
                outbound.bind(new InetSocketAddress("127.0.0.1", 0));
                outbound.setNetworkInterface(loopback);
                outbound.send(new DatagramPacket(reply, reply.length, target));
            }
        }
        System.out.println("Calimero authenticated the Rust multicast read and sent an encrypted busy report.");
    }
}
