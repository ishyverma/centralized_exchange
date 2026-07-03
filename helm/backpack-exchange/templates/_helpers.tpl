{{- define "backpack.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "backpack.labels" -}}
app.kubernetes.io/name: {{ include "backpack.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/part-of: backpack-exchange
{{- end }}
