use std::{sync::{Arc,atomic::{AtomicUsize,Ordering}},time::Duration};
use fabryk_auth::{AuthConfig,TokenValidator};
#[tokio::main]
async fn main() {
    let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr=listener.local_addr().unwrap();
    let count=Arc::new(AtomicUsize::new(0));
    let observed=count.clone();
    let server=tokio::spawn(async move {
        let (stream,_)=listener.accept().await.unwrap();
        observed.fetch_add(1,Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(5)).await;
        drop(stream);
    });
    let validator=fabryk_auth_google::GoogleTokenValidator::new(format!("http://{addr}/jwks"));
    let config=AuthConfig{enabled:true,audience:"acme-client".into(),domain:"acme.example".into()};
    let token="eyJhbGciOiJSUzI1NiIsImtpZCI6InVua25vd24ifQ.e30.eA";
    let result=tokio::time::timeout(Duration::from_millis(500),validator.validate(token,&config)).await;
    assert!(result.is_err(),"expected outer guard to cancel still-pending validation: {result:?}");
    assert_eq!(count.load(Ordering::SeqCst),1,"must reach owned fixture");
    println!("STALLED_JWKS accepted1 request; validation still pending at500ms outer guard, then cancelled. This does not measure an internal deadline or prove an infinite runtime.");
    server.abort();
}
