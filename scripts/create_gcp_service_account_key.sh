#!/bin/bash
# Helper script to create a GCP service account and key for authenticated GCS access
#
# This script creates a service account with Storage Object Viewer permissions
# and downloads the key file for use with signed URLs or Docker build secrets
#
# Usage:
#   bash scripts/create_gcp_service_account_key.sh
#   # Or make it executable and run directly:
#   chmod +x scripts/create_gcp_service_account_key.sh
#   ./scripts/create_gcp_service_account_key.sh

set -euo pipefail

PROJECT_ID="${GCP_PROJECT:-microscaler}"
SERVICE_ACCOUNT_NAME="asus-repo-access"
SERVICE_ACCOUNT_EMAIL="${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
KEY_FILE="asus-repo-service-account-key.json"

echo "🔧 Creating GCP Service Account for ASUS Repository Access"
echo "============================================================"
echo "Project: ${PROJECT_ID}"
echo "Service Account: ${SERVICE_ACCOUNT_NAME}"
echo ""

# Check if gcloud is available
if ! command -v gcloud &> /dev/null; then
    echo "❌ Error: gcloud CLI is not installed"
    echo "Install it from: https://cloud.google.com/sdk/docs/install"
    exit 1
fi

# Set project
echo "📋 Setting GCP project to ${PROJECT_ID}..."
gcloud config set project "${PROJECT_ID}" || {
    echo "❌ Error: Failed to set project. Make sure you're authenticated:"
    echo "   gcloud auth login"
    exit 1
}

# Check if service account already exists
if gcloud iam service-accounts describe "${SERVICE_ACCOUNT_EMAIL}" &>/dev/null; then
    echo "✅ Service account already exists: ${SERVICE_ACCOUNT_EMAIL}"
else
    echo "📝 Creating service account: ${SERVICE_ACCOUNT_NAME}..."
    gcloud iam service-accounts create "${SERVICE_ACCOUNT_NAME}" \
        --display-name="ASUS Repository Access" \
        --description="Service account for accessing ASUS Ascent repository in GCS" \
        --project="${PROJECT_ID}" || {
        echo "❌ Error: Failed to create service account"
        exit 1
    }
    echo "✅ Service account created"
fi

# Grant Storage Object Viewer role to the service account
echo "🔐 Granting Storage Object Viewer permissions..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/storage.objectViewer" || {
    echo "❌ Error: Failed to grant permissions"
    exit 1
}
echo "✅ Permissions granted"

# Create and download key
if [ -f "${KEY_FILE}" ]; then
    echo "⚠️  Warning: ${KEY_FILE} already exists"
    read -p "Do you want to overwrite it? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "❌ Aborted. Keeping existing key file."
        exit 0
    fi
    rm -f "${KEY_FILE}"
fi

echo "🔑 Creating and downloading service account key..."
gcloud iam service-accounts keys create "${KEY_FILE}" \
    --iam-account="${SERVICE_ACCOUNT_EMAIL}" \
    --project="${PROJECT_ID}" || {
    echo "❌ Error: Failed to create key"
    exit 1
}

echo ""
echo "✅ Success! Service account key created: ${KEY_FILE}"
echo ""
echo "📋 Next steps:"
echo "   1. Keep this key file secure - never commit it to version control"
echo "   2. Add ${KEY_FILE} to .gitignore"
echo "   3. Use it for signed URLs:"
echo "      gsutil signurl -d 1h ${KEY_FILE} gs://asus-ascent-gb10/7.2.3/dists/noble/Release"
echo "   4. Or use it with Docker build secrets:"
echo "      docker build --secret id=gcp_credentials,src=${KEY_FILE} ..."
echo ""
echo "⚠️  Security reminder:"
echo "   - Store this key securely"
echo "   - Rotate keys regularly"
echo "   - Use least privilege (this key only has Storage Object Viewer)"
echo ""

