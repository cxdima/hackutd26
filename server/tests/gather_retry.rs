use firstring::{
    calls::actor::{App, Input},
    config::Config,
};
#[tokio::test]
async fn one_gather_callback_can_claim_work_and_retries_receive_the_cached_response() {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    let a = App::new(c).await.unwrap();
    let (h, s) = a
        .incoming("CA00000000000000000000000000000001")
        .await
        .unwrap();
    let nonce = s["relay_token"].as_str().unwrap().to_string();
    let first = h
        .send(Input::GatherClaim {
            nonce: nonce.clone(),
        })
        .await
        .unwrap();
    assert_eq!(first["fresh"], true);
    let second = h
        .send(Input::GatherClaim {
            nonce: nonce.clone(),
        })
        .await
        .unwrap();
    assert_eq!(second["fresh"], false);
    h.send(Input::GatherComplete {
        nonce: nonce.clone(),
        twiml: "<Response><Say>One response</Say></Response>".into(),
    })
    .await
    .unwrap();
    let retry = h.send(Input::GatherClaim { nonce }).await.unwrap();
    assert_eq!(
        retry["twiml"],
        "<Response><Say>One response</Say></Response>"
    );
    assert!(
        h.send(Input::GatherClaim {
            nonce: "unknown".into()
        })
        .await
        .is_err()
    );
}
