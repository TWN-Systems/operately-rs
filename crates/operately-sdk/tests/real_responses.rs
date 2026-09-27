//! Deserialization tests against real response bodies captured from a live Operately
//! deployment (projects.tas.twn.network) during actual `people.list` / `tasks.create` calls
//! earlier in the same session that built this crate — not synthetic fixtures.

#[test]
fn people_list_response_deserializes() {
    // Captured verbatim from a real `GET /api/external/v1/people/list` response.
    let body = r#"{"people":[{"__typename":"person","avatar_blob_id":null,"avatar_url":null,"daily_summary_delivery_time":"18:00","description":null,"dismissed_product_release_id":"https://operately.com/releases/v190","email":"clay@twn.systems","email_preference":"buffered","email_window_minutes":5,"full_name":"Clay Townsend","has_open_invitation":null,"id":"clay-townsend-G1xl043OshErU86XpxMbeP","invite_link":null,"manager":null,"notify_about_assignments":true,"notify_on_mention":true,"peers":null,"permissions":null,"reports":null,"send_daily_summary":false,"show_dev_bar":false,"suspended":false,"time_format":"automatic","timezone":null,"title":"Founder","type":"human"},{"__typename":"person","avatar_blob_id":null,"avatar_url":null,"daily_summary_delivery_time":"18:00","description":null,"dismissed_product_release_id":null,"email":"kia@autonoma-mail.ptyktos.com","email_preference":"buffered","email_window_minutes":5,"full_name":"KIA - Autonoma","has_open_invitation":null,"id":"kia-autonoma-BtxxZbkyoIPNI2ZVf9HBl1","invite_link":null,"manager":null,"notify_about_assignments":true,"notify_on_mention":true,"peers":null,"permissions":null,"reports":null,"send_daily_summary":false,"show_dev_bar":false,"suspended":false,"time_format":"automatic","timezone":null,"title":null,"type":"human"}]}"#;

    let decoded: operately_sdk::PeopleListOutput =
        serde_json::from_str(body).expect("real people/list response must decode");
    let people = decoded.people.expect("people field present in this real response");
    assert_eq!(people.len(), 2);
    assert_eq!(people[0].full_name.as_deref(), Some("Clay Townsend"));
    assert_eq!(people[0].email.as_deref(), Some("clay@twn.systems"));
    assert_eq!(people[0].title.as_deref(), Some("Founder"));
    assert_eq!(people[1].title, None); // real null title, not an absent field — the whole point of this test
}

#[test]
fn tasks_create_response_deserializes() {
    // Captured verbatim from a real `POST /api/external/v1/tasks/create` response (trimmed to
    // the fields that matter for this test; the real response's description is a large nested
    // TipTap/ProseMirror doc, represented here as the OJson string type this crate uses).
    let body = r#"{"task":{"assignees":[],"available_statuses":null,"closed_at":null,"comments_count":null,"creator":null,"description":"{\"content\":[{\"content\":[{\"text\":\"Migrated from vyos-fabric#12.\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\"type\":\"doc\"}","due_date":null,"id":"twnbox-nation-state-model-Fq4FLGazf34mlnTKfjqxZ6","milestone":null,"name":"twnbox nation-state threat model documentation","permissions":null,"priority":null,"project":null,"project_space":null,"reminders":[],"size":null,"space":null,"status":{"closed":false,"color":"gray","id":"f3d7be8c-9580-4c72-ab3b-7c5fe47f2b36","index":0,"label":"Not started","value":"pending"},"subscription_list":null,"type":"project"},"updated_milestone":null}"#;

    let decoded: operately_sdk::TasksCreateOutput =
        serde_json::from_str(body).expect("real tasks/create response must decode");
    let task = decoded.task.expect("task field present in this real response");
    assert_eq!(task.name.as_deref(), Some("twnbox nation-state threat model documentation"));
    assert_eq!(task.id.as_deref(), Some("twnbox-nation-state-model-Fq4FLGazf34mlnTKfjqxZ6"));
    assert!(task.due_date.is_none());
}
