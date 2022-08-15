use async_std::task;
use futures::StreamExt;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{GetClosestPeersError, Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p_noise::{Keypair, X25519Spec, NoiseConfig};
use libp2p::{
    identity, upgrade,
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use std::{env, error::Error, time::Duration, HashMap};


fn main() -> Result<(), Box<dyn Error>> {

    let ver = "DEV-MAINNET";

    let PRESET_MULTIADDR = HashMap::from([
        ("MAINNET", "/dns4/p2p.zerodao.com/tcp/443/wss/p2p-webrtc-star/"),
        ("DEV-MAINNET", "/dns4/devp2p.zerodao.com/tcp/443/wss/p2p-webrtc-star/")
    ]);

    // Create a random key for ourselves.
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex protocol
    let dh_keys = Keypair::<X25519Spec>::new().into_authentic(&local_key).unwrap();
    let noise = NoiseConfig::xx(dh_keys).into_authenticated();
    let transport = WebRtcTransport::new(peer_id, vec!["stun:stun.l.google.com:19302"]);
    // let transport = builder.multiplex();

    // Create a swarm to manage peers and events.
    let mut swarm = {
        // Create a Kademlia behaviour.
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store = MemoryStore::new(local_peer_id);
        let mut behavior = Kademlia::with_config(local_peer_id, store, cfg);

        // Add the bootnodes to the local routing table. `libp2p-dns` built
        // into the `transport` resolves the `dnsaddr` when Kademlia tries
        // to dial these nodes.
        let boot_addr: Multiaddr = PRESET_MULTIADDR.get(ver).expect("Multiaddress").parse()?;
        let boot_peer = "QmXRimgxFGd8FEFRX8FvyzTG4jJTJ5pwoa3N5YDCrytASu";

        behavior.add_address(&boot_peer.parse()?, boot_addr);

        Swarm::new(transport, behavior, local_peer_id)
    };

    // Order Kademlia to search for a peer.
    let to_search: PeerId = if let Some(peer_id) = env::args().nth(1) {
        peer_id.parse()?
    } else {
        Keypair::generate_ed25519().public().into()
    };

    println!("Searching for the closest peers to {:?}", to_search);
    swarm.behaviour_mut().get_closest_peers(to_search);

    // Kick it off!
    task::block_on(async move {
        loop {
            let event = swarm.select_next_some().await;
            if let SwarmEvent::Behaviour(KademliaEvent::OutboundQueryCompleted {
                result: QueryResult::GetClosestPeers(result),
                ..
            }) = event
            {
                match result {
                    Ok(ok) => {
                        if !ok.peers.is_empty() {
                            println!("Query finished with closest peers: {:#?}", ok.peers)
                        } else {
                            // The example is considered failed as there
                            // should always be at least 1 reachable peer.
                            println!("Query finished with no closest peers.")
                        }
                    }
                    Err(GetClosestPeersError::Timeout { peers, .. }) => {
                        if !peers.is_empty() {
                            println!("Query timed out with closest peers: {:#?}", peers)
                        } else {
                            // The example is considered failed as there
                            // should always be at least 1 reachable peer.
                            println!("Query timed out with no closest peers.");
                        }
                    }
                };

                break;
            }
        }

        Ok(())
    })
}
