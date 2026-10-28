use osbeat::collectors;
use std::time::Duration;

#[tokio::test]
async fn collector_publishes_real_samples_and_stops_when_receiver_drops() {
    let (mut receiver, worker) = collectors::start();
    let first = tokio::time::timeout(Duration::from_secs(10), receiver.changed())
        .await
        .unwrap();
    first.unwrap();
    let snapshot = receiver.borrow_and_update().clone();
    assert!(snapshot.total_memory > 0);
    assert!(snapshot.uptime > 0);
    // A watch channel may replace the warm-up sample before this task is scheduled.
    assert!(snapshot.cpu.is_none_or(|cpu| (0.0..=100.0).contains(&cpu)));
    tokio::time::timeout(Duration::from_secs(10), receiver.changed())
        .await
        .unwrap()
        .unwrap();
    let snapshot = receiver.borrow_and_update().clone();
    assert!(snapshot.cpu.is_some_and(|cpu| (0.0..=100.0).contains(&cpu)));
    assert!(snapshot.own_memory > 0);
    assert!(snapshot.own_cpu.is_some());
    drop(receiver);
    tokio::time::timeout(Duration::from_secs(2), worker)
        .await
        .unwrap()
        .unwrap();
}
