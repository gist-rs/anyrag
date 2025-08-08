#!/bin/bash

# A script to build and deploy the anyrag-server to Google Cloud Run.
#
# This script automates all necessary steps:
# 1. Checks for required tools and files.
# 2. Determines the Google Cloud Project ID from an argument or internal variable.
# 3. Enables required Google Cloud services and configures Cloud Build permissions.
# 4. Creates a secret in Google Secret Manager for the AI_API_KEY for security.
# 5. Creates a dedicated service account for the Cloud Run service to run as.
# 6. Grants the necessary BigQuery and Secret Manager permissions to the service account.
# 7. Submits the Docker build to Google Cloud Build using cloudbuild.yaml.
# 8. Deploys the built image to Google Cloud Run with the correct configuration.

set -e  # Exit immediately if a command exits with a non-zero status.
set -u  # Treat unset variables as an error.
set -o pipefail # Exit on pipe failure

# --- Script Configuration ---

# Set your Google Cloud Project ID here as a fallback,
# or pass it as the first argument to the script.
# e.g., ./deploy.sh my-gcp-project-id
PROJECT_ID=""

# --- You can modify these default values if needed ---
SERVICE_NAME="anyrag-server"
REGION="asia-northeast1" # Choose a region: https://cloud.google.com/run/docs/locations
ENV_FILE="crates/server/.env"
SERVICE_ACCOUNT_NAME="anyrag-runner"
SECRET_NAME="ai-api-key"

# --- Helper for logging ---
info() {
    echo -e "\033[1;34m[INFO]\033[0m $1"
}
error() {
    echo -e "\033[1;31m[ERROR]\033[0m $1" >&2
    exit 1
}

# --- Pre-flight Checks ---
info "Running pre-flight checks..."

# Handle PROJECT_ID from command-line argument
if [ -n "${1-}" ]; then
    PROJECT_ID=$1
    info "Using Project ID from command-line argument: $PROJECT_ID"
fi

if ! command -v gcloud &> /dev/null; then
    error "gcloud command not found. Please install the Google Cloud SDK and initialize it."
fi
if [ ! -f "$ENV_FILE" ]; then
    error "Environment file not found at '$ENV_FILE'. Please create it from the example."
fi
if [ -z "$PROJECT_ID" ]; then
    error "PROJECT_ID is not set. Pass it as an argument (./deploy.sh your-project-id) or set the PROJECT_ID variable at the top of this script."
fi

# Load API Key from the .env file
AI_API_KEY=$(grep -E '^AI_API_KEY=' "$ENV_FILE" | cut -d '=' -f2- | tr -d '"' | tr -d "'")
if [ -z "$AI_API_KEY" ]; then
    error "AI_API_KEY is not set or is empty in your '$ENV_FILE' file."
fi


# --- Main Script ---

info "Step 1: Authenticating and setting up project '$PROJECT_ID'..."
# Check for active gcloud account and login only if necessary.
if ! gcloud auth list --filter=status:ACTIVE --format="value(account)" | grep -q "."; then
    info "You are not logged into gcloud. Kicking off the authentication process..."
    gcloud auth login
else
    ACTIVE_ACCOUNT=$(gcloud config get-value account)
    info "Already logged in as $ACTIVE_ACCOUNT."
fi
gcloud config set project "$PROJECT_ID"
info "gcloud project set to '$PROJECT_ID'."

info "Step 2: Enabling required Google Cloud services..."
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  secretmanager.googleapis.com \
  iam.googleapis.com \
  artifactregistry.googleapis.com

info "Step 3: Granting Cloud Build service account necessary permissions..."
PROJECT_NUMBER=$(gcloud projects describe "$PROJECT_ID" --format="value(projectNumber)")
CLOUDBUILD_SA="service-${PROJECT_NUMBER}@gcp-sa-cloudbuild.iam.gserviceaccount.com"
gcloud projects add-iam-policy-binding "$PROJECT_ID" \
    --member="serviceAccount:$CLOUDBUILD_SA" \
    --role="roles/cloudbuild.serviceAgent" \
    --quiet
info "Cloud Build service account permissions granted."

info "Step 4: Storing AI_API_KEY securely in Secret Manager..."
if gcloud secrets describe "$SECRET_NAME" &> /dev/null; then
    info "Secret '$SECRET_NAME' already exists. Adding a new version."
else
    info "Secret '$SECRET_NAME' not found. Creating it..."
    gcloud secrets create "$SECRET_NAME" --replication-policy="automatic"
fi
echo -n "$AI_API_KEY" | gcloud secrets versions add "$SECRET_NAME" --data-file=-
info "Successfully stored AI_API_KEY in Secret Manager."


info "Step 5: Setting up dedicated IAM Service Account for the Cloud Run service..."
SA_EMAIL="${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
if gcloud iam service-accounts describe "$SA_EMAIL" &> /dev/null; then
    info "Service account '$SERVICE_ACCOUNT_NAME' already exists."
else
    info "Creating service account '$SERVICE_ACCOUNT_NAME'..."
    gcloud iam service-accounts create "$SERVICE_ACCOUNT_NAME" \
        --display-name="AnyRag Service Runner"
fi

info "Granting required permissions to the service account..."
gcloud secrets add-iam-policy-binding "$SECRET_NAME" \
    --member="serviceAccount:$SA_EMAIL" \
    --role="roles/secretmanager.secretAccessor" \
    --condition=None --quiet
gcloud projects add-iam-policy-binding "$PROJECT_ID" \
    --member="serviceAccount:$SA_EMAIL" \
    --role="roles/bigquery.jobUser" --quiet
gcloud projects add-iam-policy-binding "$PROJECT_ID" \
    --member="serviceAccount:$SA_EMAIL" \
    --role="roles/bigquery.dataViewer" --quiet
info "Permissions granted."


info "Step 6: Submitting the build to Google Cloud Build..."
IMAGE_TAG="gcr.io/${PROJECT_ID}/${SERVICE_NAME}:latest"
gcloud builds submit . --config=cloudbuild.yaml \
    --substitutions=_SERVICE_NAME="$SERVICE_NAME"
info "Build submitted successfully. The image is now available at $IMAGE_TAG"


info "Step 7: Deploying to Cloud Run..."
# Extract environment variables from the .env file, excluding comments, empty lines,
# the API key (which is handled as a secret), and the local-only credentials variable.
ENV_VARS_FOR_GCLOUD=$(grep -vE '^#|^$|AI_API_KEY|GOOGLE_APPLICATION_CREDENTIALS' "$ENV_FILE" | tr '\n' ',' | sed 's/,$//')

gcloud run deploy "$SERVICE_NAME" \
  --image "$IMAGE_TAG" \
  --platform "managed" \
  --region "$REGION" \
  --service-account "$SA_EMAIL" \
  --allow-unauthenticated \
  --set-env-vars "$ENV_VARS_FOR_GCLOUD" \
  --set-secrets="AI_API_KEY=${SECRET_NAME}:latest" \
  --port 8080

SERVICE_URL=$(gcloud run services describe "$SERVICE_NAME" --platform managed --region "$REGION" --format 'value(status.url)')

echo ""
info "🚀 Deployment successful!"
info "Your service is available at: $SERVICE_URL"
