fn main() {
    let token = "eyJhbGciOiJSUzI1NiIsImtpZCI6InVua25vd24ifQ.e30.eA";
    let header = jsonwebtoken::decode_header(token).expect("fixture must stay on JWT route, never userinfo");
    assert_eq!(header.alg, jsonwebtoken::Algorithm::RS256);
    assert_eq!(header.kid.as_deref(), Some("unknown"));
    println!("Exact cached SDK header precheck passed: RS256, kid unknown; JWT dispatch only.");
}
