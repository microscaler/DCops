//! Unit tests for error module

#[cfg(test)]
mod tests {
    use crate::error::ControllerError;
    use kube::Error as KubeError;
    use netbox_client::NetBoxError;

    #[test]
    fn test_controller_error_display_kube() {
        let kube_err = KubeError::Api(kube::error::ErrorResponse {
            code: 404,
            message: "Not found".to_string(),
            reason: "NotFound".to_string(),
            status: "Failure".to_string(),
        });
        let err = ControllerError::Kube(kube_err);
        let display = format!("{}", err);
        assert!(display.contains("Kubernetes error"));
    }

    #[test]
    fn test_controller_error_display_netbox() {
        let netbox_err = NetBoxError::Api("API error".to_string());
        let err = ControllerError::NetBox(netbox_err);
        let display = format!("{}", err);
        assert!(display.contains("NetBox error"));
    }

    #[test]
    fn test_controller_error_display_ippool_not_found() {
        let err = ControllerError::IPPoolNotFound("test-pool".to_string());
        let display = format!("{}", err);
        assert!(display.contains("IPPool not found"));
        assert!(display.contains("test-pool"));
    }

    #[test]
    fn test_controller_error_display_invalid_config() {
        let err = ControllerError::InvalidConfig("Invalid configuration".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid configuration"));
    }

    #[test]
    fn test_controller_error_display_prefix_not_found() {
        let err = ControllerError::PrefixNotFound("test-prefix".to_string());
        let display = format!("{}", err);
        assert!(display.contains("NetBox prefix not found"));
        assert!(display.contains("test-prefix"));
    }

    #[test]
    fn test_controller_error_display_allocation_failed() {
        let err = ControllerError::AllocationFailed("Allocation failed".to_string());
        let display = format!("{}", err);
        assert!(display.contains("IP allocation failed"));
        assert!(display.contains("Allocation failed"));
    }

    #[test]
    fn test_controller_error_from_kube_error() {
        let kube_err = KubeError::Api(kube::error::ErrorResponse {
            code: 404,
            message: "Not found".to_string(),
            reason: "NotFound".to_string(),
            status: "Failure".to_string(),
        });
        let err: ControllerError = kube_err.into();
        match err {
            ControllerError::Kube(_) => {}
            _ => panic!("Expected Kube variant"),
        }
    }

    #[test]
    fn test_controller_error_from_netbox_error() {
        let netbox_err = NetBoxError::Api("API error".to_string());
        let err: ControllerError = netbox_err.into();
        match err {
            ControllerError::NetBox(_) => {}
            _ => panic!("Expected NetBox variant"),
        }
    }
}

