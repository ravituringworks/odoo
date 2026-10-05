use super::*;
use std::sync::Mutex;

type Logged = (String, String, Vec<(String, String)>, Option<J>, Vec<(String, String)>);
/// Records requests; answers from a script of (method, url-substring, status, body), first match wins.
struct Mock { log: Mutex<Vec<Logged>>, replies: Vec<(&'static str, &'static str, u16, String)> }
impl Mock { fn new(r: Vec<(&'static str, &'static str, u16, &str)>) -> Mock { Mock { log: Default::default(), replies: r.into_iter().map(|(m, u, s, b)| (m, u, s, b.to_string())).collect() } }
    fn last(&self) -> usize { self.log.lock().unwrap().len() - 1 }
    fn urls(&self) -> Vec<String> { self.log.lock().unwrap().iter().map(|l| format!("{} {}", l.0, l.1)).collect() }
    fn body(&self, i: usize) -> J { self.log.lock().unwrap()[i].3.clone().unwrap_or(J::Null) } }
impl Http for Mock {
    fn send(&self, r: &HttpReq) -> Result<(u16, String)> {
        self.log.lock().unwrap().push((r.method.into(), r.url.clone(), r.form.clone(), r.json.clone(), r.headers.clone()));
        let hit = self.replies.iter().find(|(m, k, _, _)| (*m == "*" || *m == r.method) && r.url.contains(k)).ok_or_else(|| OdooError::User(format!("unexpected {} {}", r.method, r.url)))?;
        Ok((hit.2, hit.3.clone()))
    }
}
fn creds() -> Creds { Creds::new("sk_test_SECRET", "https://stripe.test", "https://adyen.test") }
fn run(h: &Mock, cfg: &MethodCfg, cents: i64) -> Tx { start(h, &creds(), cfg, "tx0123456789abcdef", cents, "usd", "Order 7", "Ann").unwrap() }
fn settle(h: &Mock, cfg: &MethodCfg, tx: &mut Tx) { poll(h, &creds(), cfg, tx) }

#[test]
fn simulator_approves_after_a_moment_and_declines_special_amounts() {
    let h = Mock::new(vec![]); let sim = MethodCfg::new("simulator", &[]);
    let mut ok = run(&h, &sim, 1250); settle(&h, &sim, &mut ok); assert_eq!(ok.status, "pending");
    ok.created = Some(Instant::now() - Duration::from_secs(2)); settle(&h, &sim, &mut ok); assert_eq!((ok.status.as_str(), ok.last4.as_str()), ("succeeded", "4242"));
    let mut bad = run(&h, &sim, 1013); bad.created = Some(Instant::now() - Duration::from_secs(2)); settle(&h, &sim, &mut bad); assert_eq!(bad.status, "failed");
    let mut c = run(&h, &sim, 500); cancel(&h, &creds(), &sim, &mut c, "t3"); assert_eq!(c.status, "cancelled");
    assert!(start(&h, &creds(), &sim, "t4", 0, "usd", "x", "").is_err());
    assert!(start(&h, &creds(), &MethodCfg::new("six", &[]), "t5", 100, "usd", "x", "").unwrap_err().to_string().contains("not available"));
    assert_eq!(refund(&h, &creds(), &sim, "", 500, "usd").unwrap(), "sim-refund-500");
}

#[test]
fn stripe_creates_an_intent_hands_it_to_the_reader_follows_it_to_success_and_refunds_it() {
    let h = Mock::new(vec![("GET", "/v1/payment_intents/pi_1", 200, r#"{"status":"succeeded","latest_charge":{"payment_method_details":{"card_present":{"brand":"mastercard","last4":"4444"}}}}"#),
        ("POST", "/v1/payment_intents", 200, r#"{"id":"pi_1"}"#), ("POST", "/process_payment_intent", 200, r#"{"id":"tmr_123"}"#), ("GET", "/v1/terminal/readers/tmr_123", 200, r#"{"action":{"status":"succeeded"}}"#),
        ("POST", "/v1/refunds", 200, r#"{"id":"re_1"}"#)]);
    let cfg = MethodCfg::new("stripe", &[("stripe_serial_number", "tmr_123")]);
    let mut tx = run(&h, &cfg, 2550);
    { let log = h.log.lock().unwrap(); assert_eq!(log[0].1, "https://stripe.test/v1/payment_intents");
      assert!(log[0].2.contains(&("amount".into(), "2550".into())) && log[0].2.contains(&("payment_method_types[]".into(), "card_present".into())));
      assert_eq!(log[1].1, "https://stripe.test/v1/terminal/readers/tmr_123/process_payment_intent"); }
    settle(&h, &cfg, &mut tx); assert_eq!((tx.status.as_str(), tx.brand.as_str(), tx.last4.as_str(), tx.refref.as_str()), ("succeeded", "mastercard", "4444", "pi_1"));
    assert_eq!(refund(&h, &creds(), &cfg, &tx.refref, 1000, "usd").unwrap(), "re_1");
    let log = h.log.lock().unwrap(); let r = log.last().unwrap(); assert_eq!(r.1, "https://stripe.test/v1/refunds"); assert!(r.2.contains(&("payment_intent".into(), "pi_1".into())) && r.2.contains(&("amount".into(), "1000".into())));
}

#[test]
fn stripe_failures_and_cancellation_do_not_leak_the_key() {
    let cfg = MethodCfg::new("stripe", &[("stripe_serial_number", "tmr_123")]);
    let failed = Mock::new(vec![("POST", "/v1/payment_intents", 200, r#"{"id":"pi_2"}"#), ("POST", "/process_payment_intent", 200, "{}"), ("GET", "/v1/terminal/readers/tmr_123", 200, r#"{"action":{"status":"failed","failure_message":"Card declined sk_test_SECRET"}}"#)]);
    let mut tx = run(&failed, &cfg, 100); settle(&failed, &cfg, &mut tx); assert_eq!(tx.status, "failed"); assert!(!tx.message.contains("SECRET") && tx.message.contains("declined"));
    let denied = Mock::new(vec![("POST", "/v1/payment_intents", 401, r#"{"error":{"message":"Invalid API Key provided: sk_test_SECRET"}}"#)]);
    let e = start(&denied, &creds(), &cfg, "b", 100, "usd", "o", "").unwrap_err().to_string(); assert!(e.contains("Invalid API Key") && !e.contains("sk_test_SECRET"), "{e}");
    let ok = Mock::new(vec![("POST", "/v1/payment_intents", 200, r#"{"id":"pi_3"}"#), ("POST", "/process_payment_intent", 200, "{}"), ("POST", "/cancel_action", 200, "{}")]);
    let mut t = run(&ok, &cfg, 100); cancel(&ok, &creds(), &cfg, &mut t, "c"); assert_eq!(t.status, "cancelled"); assert!(ok.urls().iter().any(|l| l.ends_with("/cancel_action")));
    assert!(start(&ok, &Creds::new("", "https://stripe.test", ""), &cfg, "d", 100, "usd", "o", "").is_err());
    assert!(start(&ok, &creds(), &MethodCfg::new("stripe", &[]), "e", 100, "usd", "o", "").is_err());
}

#[test]
fn adyen_sends_a_terminal_api_request_parses_the_result_and_reverses_it() {
    let m = MethodCfg::new("adyen", &[("adyen_api_key", "AQE_SECRET"), ("adyen_terminal_identifier", "P400Plus-123")]);
    let good = Mock::new(vec![("POST", "/sync", 200, r#"{"SaleToPOIResponse":{"PaymentResponse":{"Response":{"Result":"Success"},"POIData":{"POITransactionID":{"TransactionID":"T123","TimeStamp":"2026-01-01T10:00:00Z"}},"PaymentResult":{"PaymentInstrumentData":{"CardData":{"PaymentBrand":"visa","MaskedPan":"411111......1111"}}}}}}"#)]);
    let o = adyen_charge(&good, &m, "https://adyen.test", "SVC0000001", 1999, "eur", "Order 9");
    assert_eq!((o.status.as_str(), o.brand.as_str(), o.last4.as_str(), o.reference.as_str()), ("succeeded", "visa", "1111", "T123|2026-01-01T10:00:00Z"));
    let b = good.body(0); assert_eq!(b["SaleToPOIRequest"]["MessageHeader"]["POIID"], "P400Plus-123"); assert_eq!(b["SaleToPOIRequest"]["PaymentRequest"]["PaymentTransaction"]["AmountsReq"]["RequestedAmount"], 19.99);
    let o = parse_adyen(r#"{"SaleToPOIResponse":{"PaymentResponse":{"Response":{"Result":"Failure","AdditionalResponse":"message%3DCustomer+cancelled+AQE_SECRET"}}}}"#, "AQE_SECRET");
    assert_eq!(o.status, "failed"); assert!(o.message.contains("Customer cancelled") && !o.message.contains("SECRET"), "{}", o.message);
    let down = Mock::new(vec![("POST", "/sync", 401, "bad AQE_SECRET")]); let o = adyen_charge(&down, &m, "https://adyen.test", "S", 100, "usd", "o"); assert_eq!(o.status, "failed"); assert!(!o.message.contains("AQE_SECRET"));
    assert!(start(&down, &creds(), &MethodCfg::new("adyen", &[]), "x", 100, "usd", "o", "").is_err());
    let rev = Mock::new(vec![("POST", "/sync", 200, r#"{"SaleToPOIResponse":{"ReversalResponse":{"Response":{"Result":"Success"}}}}"#)]);
    assert_eq!(refund(&rev, &creds(), &m, "T123|2026-01-01T10:00:00Z", 500, "eur").unwrap(), "T123-reversal");
    let b = rev.body(0); assert_eq!(b["SaleToPOIRequest"]["ReversalRequest"]["OriginalPOITransaction"]["POITransactionID"]["TransactionID"], "T123"); assert_eq!(b["SaleToPOIRequest"]["ReversalRequest"]["ReversedAmount"], 5.0);
}

#[test]
fn mercado_pago_finds_the_device_creates_an_intent_polls_cancels_and_refunds() {
    let cfg = MethodCfg::new("mercado_pago", &[("mp_bearer_token", "APP_USR_SECRET"), ("mp_id_point_smart", "N950")]);
    let h = Mock::new(vec![("GET", "/devices", 200, r#"{"devices":[{"id":"PAX_A910__SMARTPOS1"},{"id":"NEWLAND_N950__N950NCB801"}]}"#), ("POST", "/payment-intents", 201, r#"{"id":"pi-7","state":"OPEN"}"#),
        ("GET", "/payment-intents/pi-7", 200, r#"{"id":"pi-7","state":"FINISHED","payment":{"id":555}}"#), ("GET", "/v1/payments/555", 200, r#"{"status":"approved","payment_method_id":"master","card":{"last_four_digits":"1234"}}"#),
        ("DELETE", "/payment-intents/pi-7", 200, "{}"), ("POST", "/v1/payments/555/refunds", 201, r#"{"id":99}"#)]);
    let mut tx = run(&h, &cfg, 1500);
    assert!(h.urls()[1].contains("/devices/NEWLAND_N950__N950NCB801/payment-intents")); assert_eq!(h.body(1)["amount"], 1500); assert_eq!(h.body(1)["additional_info"]["print_on_terminal"], true);
    assert!(h.log.lock().unwrap()[0].4.contains(&("Authorization".into(), "Bearer APP_USR_SECRET".into())));
    settle(&h, &cfg, &mut tx); assert_eq!((tx.status.as_str(), tx.brand.as_str(), tx.last4.as_str(), tx.refref.as_str()), ("succeeded", "master", "1234", "555"));
    assert_eq!(refund(&h, &creds(), &cfg, "555", 500, "usd").unwrap(), "99"); assert_eq!(h.body(h.last())["amount"], 5.0);
    // pending, rejected, unknown terminal, cancel
    let pending = Mock::new(vec![("GET", "/devices", 200, r#"{"devices":[{"id":"NEWLAND_N950__N950NCB801"}]}"#), ("POST", "/payment-intents", 201, r#"{"id":"pi-8"}"#), ("GET", "/payment-intents/pi-8", 200, r#"{"state":"ON_TERMINAL"}"#), ("DELETE", "/payment-intents/pi-8", 200, "{}")]);
    let mut t = run(&pending, &cfg, 100); settle(&pending, &cfg, &mut t); assert_eq!(t.status, "pending"); cancel(&pending, &creds(), &cfg, &mut t, "x"); assert_eq!(t.status, "cancelled"); assert!(pending.urls().last().unwrap().starts_with("DELETE"));
    let none = Mock::new(vec![("GET", "/devices", 200, r#"{"devices":[]}"#)]); assert!(start(&none, &creds(), &cfg, "z", 100, "usd", "o", "").unwrap_err().to_string().contains("not registered"));
    let denied = Mock::new(vec![("GET", "/devices", 401, r#"{"message":"invalid token"}"#)]); assert!(start(&denied, &creds(), &cfg, "z", 100, "usd", "o", "").unwrap_err().to_string().contains("rejected"));
}

#[test]
fn viva_wallet_gets_a_token_sends_a_sale_and_waits_for_the_session_result() {
    let cfg = MethodCfg::new("viva_wallet", &[("viva_wallet_client_id", "cid"), ("viva_wallet_client_secret", "SECRET"), ("viva_wallet_terminal_id", "16002169"), ("viva_wallet_test_mode", "true")]);
    let h = Mock::new(vec![("POST", "/connect/token", 200, r#"{"access_token":"tok_1"}"#), ("POST", "/transactions:sale", 200, "{}"), ("DELETE", "/sessions/", 200, "")]);
    let mut tx = run(&h, &cfg, 2000);
    let u = h.urls(); assert!(u[0].contains("demo-accounts.vivapayments.com/connect/token")); assert!(u[1].contains("demo-api.vivapayments.com/ecr/v1/transactions:sale"));
    let b = h.body(1); assert_eq!((b["amount"].as_i64(), b["currencyCode"].as_str(), b["terminalId"].as_str(), b["cashRegisterId"].as_str()), (Some(2000), Some("840"), Some("16002169"), Some("Ann")));
    assert!(h.log.lock().unwrap()[0].4.iter().any(|(k, v)| k == "Authorization" && v.starts_with("Basic ")));
    let pending = Mock::new(vec![("GET", "/sessions/", 404, r#"{"detail":"not found"}"#)]); settle(&pending, &cfg, &mut tx); assert_eq!(tx.status, "pending");
    let done = Mock::new(vec![("GET", "/sessions/", 200, r#"{"success":true,"transactionId":"tr-1","applicationLabel":"VISA","primaryAccountNumberMasked":"4111XXXXXXXX1111"}"#)]);
    settle(&done, &cfg, &mut tx); assert_eq!((tx.status.as_str(), tx.brand.as_str(), tx.last4.as_str()), ("succeeded", "VISA", "1111"));
    let mut t2 = run(&h, &cfg, 2000); let declined = Mock::new(vec![("GET", "/sessions/", 200, r#"{"success":false,"message":"Declined SECRET"}"#)]); settle(&declined, &cfg, &mut t2); assert_eq!(t2.status, "failed"); assert!(!t2.message.contains("SECRET"));
    let mut t3 = run(&h, &cfg, 2000); cancel(&h, &creds(), &cfg, &mut t3, "x"); assert!(h.urls().last().unwrap().contains("sessions/") && h.urls().last().unwrap().contains("cashRegisterId=Ann"));
    let badtoken = Mock::new(vec![("POST", "/connect/token", 401, r#"{"error":"invalid_client"}"#)]); assert!(start(&badtoken, &creds(), &cfg, "z", 100, "usd", "o", "").unwrap_err().to_string().contains("token"));
    assert!(refund(&h, &creds(), &cfg, "tr-1", 100, "usd").is_err() && !can_refund("viva_wallet"));
}

#[test]
fn razorpay_pushes_to_the_device_and_follows_the_p2p_request() {
    let cfg = MethodCfg::new("razorpay", &[("razorpay_tid", "7000012300"), ("razorpay_username", "1234500121"), ("razorpay_api_key", "KEY_SECRET"), ("razorpay_allowed_payment_modes", "card")]);
    let h = Mock::new(vec![("POST", "/pay", 200, r#"{"success":true,"p2pRequestId":"12345"}"#), ("POST", "/status", 200, r#"{"success":true,"status":"AUTHORIZED","messageCode":"P2P_DEVICE_TXN_DONE","externalRefNumber":"Order 7","cardLastFourDigit":"4242","paymentCardBrand":"VISA","txnId":"tx9","authCode":"A1"}"#), ("POST", "/cancel", 200, r#"{"success":true}"#)]);
    let mut tx = run(&h, &cfg, 1999); let b = h.body(0);
    assert_eq!((b["pushTo"]["deviceId"].as_str(), b["mode"].as_str(), b["amount"].as_f64(), b["externalRefNumber"].as_str()), (Some("7000012300|ezetap_android"), Some("CARD"), Some(19.99), Some("Order 7")));
    settle(&h, &cfg, &mut tx); assert_eq!((tx.status.as_str(), tx.brand.as_str(), tx.last4.as_str(), tx.refref.as_str()), ("succeeded", "VISA", "4242", "tx9"));
    assert_eq!(h.body(1)["origP2pRequestId"], "12345");
    let mismatch = Mock::new(vec![("POST", "/status", 200, r#"{"success":true,"status":"AUTHORIZED","messageCode":"P2P_DEVICE_TXN_DONE","externalRefNumber":"OTHER"}"#)]); let mut t = run(&h, &cfg, 100); settle(&mismatch, &cfg, &mut t); assert_eq!(t.status, "failed"); assert!(t.message.contains("mismatch"));
    let queued = Mock::new(vec![("POST", "/status", 200, r#"{"success":true,"status":"QUEUED","messageCode":"P2P_STATUS_QUEUED"}"#)]); let mut t = run(&h, &cfg, 100); settle(&queued, &cfg, &mut t); assert_eq!(t.status, "pending");
    let cancelled = Mock::new(vec![("POST", "/status", 200, r#"{"success":true,"status":"FAILED","messageCode":"P2P_DEVICE_CANCELED","message":"Cancelled by customer"}"#)]); let mut t = run(&h, &cfg, 100); settle(&cancelled, &cfg, &mut t); assert_eq!(t.status, "failed");
    let mut t = run(&h, &cfg, 100); cancel(&h, &creds(), &cfg, &mut t, "x"); assert!(h.urls().last().unwrap().ends_with("/cancel"));
    let rejected = Mock::new(vec![("POST", "/pay", 200, r#"{"success":false,"errorCode":"E1","errorMessage":"Device offline KEY_SECRET"}"#)]); let e = start(&rejected, &creds(), &cfg, "z", 100, "inr", "o", "").unwrap_err().to_string(); assert!(e.contains("offline") && !e.contains("KEY_SECRET"));
}

#[test]
fn paytm_signs_requests_with_aes_cbc_and_follows_the_status_codes() {
    // signature structure: base64 of whole AES blocks, deterministic for a given salt
    let params: BTreeMap<String, String> = [("a", "1"), ("b", "2")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let sig = paytm_sig(&params, "B1o6Ivjy8L1@abc9", "abcd"); { use base64::{engine::general_purpose::STANDARD, Engine}; assert_eq!(STANDARD.decode(&sig).unwrap().len() % 16, 0); }
    assert_eq!(sig, paytm_sig(&params, "B1o6Ivjy8L1@abc9", "abcd")); assert_ne!(sig, paytm_sig(&params, "B1o6Ivjy8L1@abc9", "abce")); assert_ne!(sig, paytm_sig(&params, "B1o6Ivjy8L1@abc8", "abcd"));
    assert!(drivers::paytm_signature(&params, "short", "abcd").is_err());
    assert_eq!(drivers::fmt_ist(0), "1970-01-01 05:30:00"); assert_eq!(drivers::fmt_ist(1_790_000_000), "2026-09-21 19:43:20");
    let cfg = MethodCfg::new("paytm", &[("paytm_mid", "MID1"), ("paytm_tid", "70000123"), ("paytm_merchant_key", "B1o6Ivjy8L1@abc9"), ("allowed_payment_modes", "card"), ("accept_payment", "auto")]);
    let h = Mock::new(vec![("POST", "payment/request", 200, r#"{"body":{"resultInfo":{"resultCode":"A","resultMsg":"Accepted"}}}"#), ("POST", "V2/payment/status", 200, "{}")]);
    let mut tx = run(&h, &cfg, 5000); let b = h.body(0);
    assert_eq!((b["body"]["transactionAmount"].as_str(), b["body"]["autoAccept"].as_str(), b["body"]["paymentMode"].as_str(), b["body"]["paytmMid"].as_str()), (Some("5000"), Some("True"), Some("CARD"), Some("MID1")));
    assert!(b["head"]["checksum"].as_str().unwrap().len() > 20 && b["head"]["channelId"] == "EDC");
    let rid = tx.aux["rid"].clone();
    let pend = Mock::new(vec![("POST", "V2/payment/status", 200, r#"{"body":{"resultInfo":{"resultCode":"P"}}}"#)]); settle(&pend, &cfg, &mut tx); assert_eq!(tx.status, "pending");
    let ok = Mock::new(vec![("POST", "V2/payment/status", 200, &format!(r#"{{"body":{{"resultInfo":{{"resultCode":"S"}},"merchantReferenceNo":"{rid}","issuerMaskCardNo":"4111XXXX1111","cardScheme":"VISA","merchantTransactionId":"mt1"}}}}"#))]); settle(&ok, &cfg, &mut tx);
    assert_eq!((tx.status.as_str(), tx.last4.as_str(), tx.brand.as_str()), ("succeeded", "1111", "VISA"));
    let mut t2 = run(&h, &cfg, 100); let f = Mock::new(vec![("POST", "V2/payment/status", 200, r#"{"body":{"resultInfo":{"resultCode":"F","resultMsg":"Declined"}}}"#)]); settle(&f, &cfg, &mut t2); assert_eq!(t2.status, "failed");
    let declined = Mock::new(vec![("POST", "payment/request", 200, r#"{"body":{"resultInfo":{"resultCode":"F","resultMsg":"Terminal busy"}}}"#)]); assert!(start(&declined, &creds(), &cfg, "z", 100, "inr", "o", "").unwrap_err().to_string().contains("busy"));
}
fn paytm_sig(p: &BTreeMap<String, String>, key: &str, salt: &str) -> String { drivers::paytm_signature(p, key, salt).unwrap() }

#[test]
fn pine_labs_uploads_the_bill_polls_the_cloud_status_and_cancels() {
    let cfg = MethodCfg::new("pine_labs", &[("pine_labs_merchant", "M1"), ("pine_labs_store", "S1"), ("pine_labs_client", "C1"), ("pine_labs_security_token", "TOKEN_SECRET"), ("pine_labs_allowed_payment_mode", "upi")]);
    let h = Mock::new(vec![("POST", "UploadBilledTransaction", 200, r#"{"ResponseCode":0,"ResponseMessage":"APPROVED","PlutusTransactionReferenceID":8001}"#),
        ("POST", "GetCloudBasedTxnStatus", 200, r#"{"ResponseCode":0,"ResponseMessage":"TXN APPROVED","PlutusTransactionReferenceID":8001,"TransactionData":[{"Tag":"Card Number","Value":"4111111111111111"},{"Tag":"Card Type","Value":"VISA"},{"Tag":"TransactionLogId","Value":"L77"}]}"#),
        ("POST", "CancelTransactionForced", 200, r#"{"ResponseCode":0,"ResponseMessage":"APPROVED"}"#)]);
    let mut tx = run(&h, &cfg, 7500); let b = h.body(0);
    assert_eq!((b["Amount"].as_i64(), b["MerchantID"].as_str(), b["AllowedPaymentMode"].as_i64(), b["SecurityToken"].as_str()), (Some(7500), Some("M1"), Some(10), Some("TOKEN_SECRET")));
    settle(&h, &cfg, &mut tx); assert_eq!((tx.status.as_str(), tx.brand.as_str(), tx.last4.as_str(), tx.refref.as_str()), ("succeeded", "VISA", "1111", "L77"));
    let up = Mock::new(vec![("POST", "GetCloudBasedTxnStatus", 200, r#"{"ResponseCode":0,"ResponseMessage":"TXN UPLOADED","TransactionData":[]}"#)]); let mut t = run(&h, &cfg, 100); settle(&up, &cfg, &mut t); assert_eq!(t.status, "pending");
    let prog = Mock::new(vec![("POST", "GetCloudBasedTxnStatus", 200, r#"{"ResponseCode":1001,"ResponseMessage":"TXN IN PROGRESS"}"#)]); settle(&prog, &cfg, &mut t); assert_eq!(t.status, "pending");
    let bad = Mock::new(vec![("POST", "GetCloudBasedTxnStatus", 200, r#"{"ResponseCode":1,"ResponseMessage":"TXN DECLINED TOKEN_SECRET"}"#)]); settle(&bad, &cfg, &mut t); assert_eq!(t.status, "failed"); assert!(!t.message.contains("TOKEN_SECRET"));
    let mut t = run(&h, &cfg, 100); cancel(&h, &creds(), &cfg, &mut t, "x"); assert!(h.urls().last().unwrap().ends_with("CancelTransactionForced")); assert_eq!(h.body(h.last())["TakeToHomeScreen"], true);
    let rej = Mock::new(vec![("POST", "UploadBilledTransaction", 200, r#"{"ResponseCode":5,"ResponseMessage":"TERMINAL BUSY"}"#)]); assert!(start(&rej, &creds(), &cfg, "z", 100, "inr", "o", "").unwrap_err().to_string().contains("BUSY"));
}
