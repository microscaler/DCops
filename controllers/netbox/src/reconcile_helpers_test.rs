//! Unit tests for reconcile_helpers module

#[cfg(test)]
mod tests {
    use super::super::reconcile_helpers;
    use netbox_client::NestedTag;
    
    fn create_nested_tag(id: u64, name: &str) -> NestedTag {
        NestedTag {
            id,
            url: format!("http://test/api/extras/tags/{}/", id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
        }
    }
    
    fn create_tag_ref(name: &str) -> crds::NetBoxResourceReference {
        crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTag".to_string(),
            name: name.to_string(),
            namespace: None,
        }
    }
    
    #[test]
    fn test_tags_differ_empty_vs_empty() {
        let existing: Vec<NestedTag> = vec![];
        let desired: Option<Vec<crds::NetBoxResourceReference>> = None;
        
        assert!(!reconcile_helpers::tags_differ(&existing, &desired), 
            "Empty tags should not differ");
    }
    
    #[test]
    fn test_tags_differ_empty_vs_some() {
        let existing: Vec<NestedTag> = vec![];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Empty existing vs some desired should differ");
    }
    
    #[test]
    fn test_tags_differ_some_vs_empty() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired: Option<Vec<crds::NetBoxResourceReference>> = None;
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Some existing vs empty desired should differ");
    }
    
    #[test]
    fn test_tags_differ_same_tags() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![
            create_tag_ref("tag1"),
            create_tag_ref("tag2"),
        ]);
        
        assert!(!reconcile_helpers::tags_differ(&existing, &desired), 
            "Same tags should not differ");
    }
    
    #[test]
    fn test_tags_differ_different_tags() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired = Some(vec![create_tag_ref("tag2")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Different tags should differ");
    }
    
    #[test]
    fn test_tags_differ_different_order() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![
            create_tag_ref("tag2"),
            create_tag_ref("tag1"),
        ]);
        
        assert!(!reconcile_helpers::tags_differ(&existing, &desired), 
            "Tags in different order should not differ (order doesn't matter)");
    }
    
    #[test]
    fn test_tags_differ_extra_existing() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Extra existing tags should differ");
    }
    
    #[test]
    fn test_tags_differ_extra_desired() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired = Some(vec![
            create_tag_ref("tag1"),
            create_tag_ref("tag2"),
        ]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Extra desired tags should differ");
    }
    
    #[test]
    fn test_tags_differ_case_sensitive() {
        let existing = vec![create_nested_tag(1, "Tag1")];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Tags should be case-sensitive");
    }
}
