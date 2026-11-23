// Synthetic connectionless, normal-mode exchange through Calimero's secure application layer.
package io.calimero.secure;

import io.calimero.*;
import io.calimero.cemi.*;
import io.calimero.link.AbstractLink;
import io.calimero.link.medium.TPSettings;
import java.io.*;
import java.util.*;
import java.util.concurrent.*;

public final class DataSyncPeer {
    private static final IndividualAddress local = new IndividualAddress(0x1102);
    private static final IndividualAddress remote = new IndividualAddress(0x1101);
    private static final byte[] key = HexFormat.of().parseHex("000102030405060708090a0b0c0d0e0f");
    private static final BlockingQueue<byte[]> outbound = new ArrayBlockingQueue<>(2);

    static byte[] take() throws Exception {
        return Objects.requireNonNull(outbound.poll(3, TimeUnit.SECONDS), "no protected Calimero APDU");
    }
    static void deliver(OutputStream pipe, byte[] apdu) throws IOException {
        pipe.write(new CEMILDataEx(CEMILData.MC_LDATA_IND, local, remote, apdu, Priority.SYSTEM).toByteArray());
        pipe.flush();
    }
    static CEMILData read(InputStream pipe) throws Exception {
        byte[] frame = pipe.readNBytes(34);
        if (frame.length != 34) throw new EOFException();
        var packet = new CEMILDataEx(frame, 0, frame.length);
        if (packet.getMessageCode() != CEMILData.MC_LDATA_REQ ||
                !packet.getSource().equals(remote) || !packet.getDestination().equals(local))
            throw new AssertionError("unexpected source, destination or primitive");
        return packet;
    }
    public static void main(String[] args) throws Exception {
        var link = new AbstractLink<>(null, "sync-fixture", new TPSettings(local)) {
            protected void onSend(CEMILData msg, boolean wait) {}
            protected void onSend(KNXAddress dst, byte[] msg, boolean wait) {}
        };
        try (var sal = new SecureApplicationLayer(link, SerialNumber.Zero, 100, Map.of()) {
            protected byte[] securityKey(KNXAddress address) { return address.equals(remote) ? key : null; }
            protected void send(KNXAddress dst, byte[] apdu) { outbound.add(apdu); }
        }) {
            if (args.length == 0) {
                // Reproducible peer-produced inbound request for the Rust regression test.
                byte[] apdu = sal.secure(SecureApplicationLayer.SecureSyncRequest, local, remote,
                    new byte[]{0,0,0,0,0,3}, SecurityControl.of(SecurityControl.DataSecurity.AuthConf,false)).orElseThrow();
                System.out.println(HexFormat.of().formatHex(new CEMILDataEx(CEMILData.MC_LDATA_IND,
                                        local,remote,apdu,Priority.SYSTEM).toByteArray()));
                return;
            }
            var rust = new ProcessBuilder(args[0]).redirectError(ProcessBuilder.Redirect.INHERIT).start();
            try (var input = rust.getInputStream(); var output = rust.getOutputStream()) {
                sal.extract(read(input));
                deliver(output, take());
                var challenge = sal.sendSyncRequest(remote, false);
                deliver(output, take());
                sal.extract(read(input));
                challenge.get(3, TimeUnit.SECONDS);
                if (sal.lastValidSequenceNumber(false,remote) != 200)
                    throw new AssertionError("Calimero did not accept Rust's next sender sequence");
                if (rust.waitFor() != 0) throw new AssertionError("Rust sync fixture failed");
            } finally {rust.destroyForcibly();}
        } finally {link.close();}
        System.out.println("Calimero and Rust authenticated both directions of normal-mode sequence synchronization.");
    }
}
