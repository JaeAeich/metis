{{/*
Expand the name of the chart.
*/}}
{{- define "metis.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "metis.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "metis.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "metis.labels" -}}
helm.sh/chart: {{ include "metis.chart" . }}
{{ include "metis.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "metis.selectorLabels" -}}
app.kubernetes.io/name: {{ include "metis.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "metis.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "metis.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}


{{/*
Return the MongoDB Hostname
*/}}
{{- define "metis.mongodb.host" -}}
{{- if not .Values.metis.config.mongo.host }}
{{- fail "metis.config.mongo.host is required when using external MongoDB" }}
{{- end }}
{{- .Values.metis.config.mongo.host }}
{{- end -}}

{{/*
Return the MongoDB Port
*/}}
{{- define "metis.mongodb.port" -}}
{{- if not .Values.metis.config.mongo.port }}
{{- fail "metis.config.mongo.port is required when using external MongoDB" }}
{{- end }}
{{- .Values.metis.config.mongo.port }}
{{- end -}}

{{/*
Return the MongoDB Secret Name
*/}}
{{- define "metis.mongodb.secretName" -}}
{{- printf "%s-mongodb-external" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Return the Staging Secret Name
*/}}
{{- define "metis.staging.secretName" -}}
{{- printf "%s-staging-secrets" .Release.Name | trunc 63 | trimSuffix "-" }}
{{- end -}}

{{/*
Return the MongoDB Secret Password Key
*/}}
{{- define "metis.mongodb.password" -}}
{{- if not .Values.metis.config.mongo.password }}
{{- fail "metis.config.mongo.password is required when using external MongoDB" }}
{{- end }}
{{- .Values.metis.config.mongo.password -}}
{{- end -}}

{{/*
Return the MongoDB Username
*/}}
{{- define "metis.mongodb.username" -}}
{{- if not .Values.metis.config.mongo.username }}
{{- fail "metis.config.mongo.username is required when using external MongoDB" }}
{{- end }}
{{- .Values.metis.config.mongo.username -}}
{{- end -}}

{{/*
Return the MongoDB Database
*/}}
{{- define "metis.mongodb.database" -}}
{{- .Values.metis.config.mongo.database -}}
{{- end -}}

{{/*
Return the MongoDB Workflow Collection
*/}}
{{- define "metis.mongodb.workflowCollection" -}}
{{- .Values.metis.config.mongo.workflowCollection -}}
{{- end -}}

{{/* MinIO access helper templates removed - using staging credentials instead */}}

{{/*
Return the Staging URL
*/}}
{{- define "metis.staging.url" -}}
{{- if not .Values.metis.config.metel.staging.url }}
{{- fail "metis.config.metel.staging.url is required when using external S3 storage" }}
{{- end }}
{{- .Values.metis.config.metel.staging.url -}}
{{- end -}}

{{/*
Return the Staging Bucket
*/}}
{{- define "metis.staging.bucket" -}}
{{- if not .Values.metis.config.metel.staging.bucket }}
{{- fail "metis.config.metel.staging.bucket is required when using external S3 storage" }}
{{- end }}
{{- .Values.metis.config.metel.staging.bucket -}}
{{- end -}}

{{/*
Return the Staging Prefix
*/}}
{{- define "metis.staging.prefix" -}}
{{- .Values.metis.config.metel.staging.prefix -}}
{{- end -}}

{{/*
Return the Staging Type
*/}}
{{- define "metis.staging.type" -}}
{{- .Values.metis.config.metel.staging.type | default "s3" -}}
{{- end -}}

{{/*
Create a plugin configmap name
*/}}
{{- define "metis.plugin.configMapName" -}}
{{- printf "%s-plugin" (include "metis.fullname" .) }}
{{- end }}

{{/*
Create a plugin name from plugin configuration and global names
*/}}
{{- define "metis.plugin.name" -}}
{{- $plugin := .plugin -}}
{{- $global := .global -}}
{{- if $plugin.name }}
{{- printf "%s-%s" (include "metis.fullname" $global) $plugin.name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s-plugin" (include "metis.fullname" $global) $plugin.workflow_type | lower | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{/*
Plugin labels
*/}}
{{- define "metis.plugin.labels" -}}
{{- $plugin := .plugin -}}
{{- $global := .global -}}
helm.sh/chart: {{ include "metis.chart" $global }}
{{ include "metis.plugin.selectorLabels" . }}
{{- if $global.Chart.AppVersion }}
app.kubernetes.io/version: {{ $global.Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ $global.Release.Service }}
metis/component: plugin
{{- if $plugin.workflow_type }}
metis/workflow-type: {{ $plugin.workflow_type | lower }}
{{- end }}
{{- end }}

{{/*
Plugin selector labels
*/}}
{{- define "metis.plugin.selectorLabels" -}}
{{- $plugin := .plugin -}}
{{- $global := .global -}}
app.kubernetes.io/name: {{ include "metis.plugin.name" . }}
app.kubernetes.io/instance: {{ $global.Release.Name }}
{{- end }}
