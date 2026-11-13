#[test]
#[ignore = "local IPC integration check; requires permission to bind a loopback socket"]
fn a_second_launch_forwards_to_the_running_instance() {
    use athanor::services::instance::{InstanceCommand, InstanceServer, Start};
    let dir = tempfile::tempdir().unwrap();
    let Start::Primary(server, receiver) = InstanceServer::start(dir.path()).unwrap() else {
        panic!("first launch forwarded")
    };
    assert!(matches!(
        InstanceServer::start(dir.path()).unwrap(),
        Start::Forwarded
    ));
    assert_eq!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap(),
        InstanceCommand::Show
    );
    drop(server);
    assert!(matches!(
        InstanceServer::start(dir.path()).unwrap(),
        Start::Primary(..)
    ));
}
