# Aether

Simple and fast Ingress Provider and TLS termination for kubernetes.
**Currently not production-ready.**

This provider autodiscovers certificates to be used with TLS based on the configuration, rather than having to provide it in the ingress configuration.
It can be used in combination with [Cert Manager](https://cert-manager.io/) or simulair to automatically generate and serve certificates.

## Usage

### Installing the Chart.

To install Aether, clone the repository, and change directory to `chart`.
Now you can run this command to install the ingress provider.
It will create an External Loadbalancer, which will be managed by the ingress.

```
helm install aether . -n aether --create-namespace
```

### Creating the ingress.

Like all ingress providers, this ingress provider works with an ingress definition.
You can check out the [Ingress documentation](https://kubernetes.io/docs/concepts/services-networking/ingress/) for more information.
Also check out the [limitations](#limitations) sections for any deviations.

```yml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: ingress-name
  annotations:
    kubernetes.io/ingress.class: aether
spec:
  defaultBackend:
    service:
      name: default-service
      port:
        number: 80
  rules:
    - http:
        paths:
          - path: /subpath
            pathType: Prefix
            backend:
              service:
                name: second-service
                port:
                  number: 80
```

Note that there is no `tls` section in the specification.
This is because the certificates are autodiscovered from the secrets, see the next section.

### Adding a secret.

Aether TLS is configured by annotations in secrets.
Set the `aether.rs/hosts` annotation to the host(s) the secret is providing.
You can use an comma separated list here in order to provide multiple hosts.
For example, set to `example.com,aether.rs` will provide the certificate in the secret to both `example.com` and to `aether.rs`.
The type should be set to `kubernetes.io/tls`, but the name can be arbitrary.
Currently multiple secrets with the same host is not defined.
You can create the secret by applying:

```yml
apiVersion: v1
kind: Secret
type: kubernetes.io/tls
metadata:
  name: ingress-secret
  annotations:
    aether.rs/hosts: "example.com,aether.rs"
data:
  tls.crt: |
    MIIC2DCCAcCgAwIBAgIBATANBgkqh ...
  tls.key: |
    MIIEpgIBAAKCAQEA7yn3bRHQ5FHMQ ...
```

Or you can run the following commands:

```
kubectl create secret tls ingress-secret \
  --cert=path/to/cert/file \
  --key=path/to/key/file \
kubectl annotate secrets ingress-secret aether.rs/hosts=example.com,aether.rs
```

### Usage with `cert-manager`.

`cert-manager` is an Kubernetes certificate manager, and can be used to provide certificates to Aether.
First follow the installation guide on [the website](https://cert-manager.io/docs/installation/).
At least version 1.5.0 is required here, otherwise the annotations will not work.

Now you can configure `cert-manager` according to [the documentation](https://cert-manager.io/docs/configuration/).
If you are using ACME, please make sure you are using the `DNS01` challenge, as `HTTP01` will not work.
You can use this example to create an issuer when hosting your domain on Cloudflare.

```yml
apiVersion: cert-manager.io/v1
kind: Issuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: { { issuer-email } }
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - dns01:
          cloudflare:
            email: { { cloudflare-email } }
            apiKeySecretRef:
              name: cloudflare-apikey
              key: apikey
---
apiVersion: v1
kind: Secret
metadata:
  name: cloudflare-apikey
type: Opaque
stringData:
  apikey: { { cf-global-apikey } }
```

And then create the certificate, make sure to match the name to the secret created before.
Note the secretTemplate at the end of the specs, this will define for which hosts the certificate will be used.
This should be a comma separated list of hosts.

```yml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: aether-certificate
spec:
  secretName: cert-example.com
  dnsNames:
    - example.com
    - aether.rs
  privateKey:
    rotationPolicy: Always
  issuerRef:
    name: letsencrypt-prod
    kind: Issuer
  secretTemplate:
    annotations:
      aether.rs/hosts: "example.com,aether.rs"
```

This should create the certificate and you should now be able to use the ingress.

### Limitations

As this is an highly specialized Ingress Provider, there are still a lot of limitations.
Please make sure this provider suits your usecase, as it's not fully compliant with the Ingress specification (yet).
If there is missing functionality you need, please [Open an Issue](https://github.com/nexiumapp/aether/issues/new).

An non-exhaustive list of current limitations:

- It ignores the `tls` field in the specs. This is by design, but might be added in the future as an override method.
- The `host` field in the specs is currently ignored.
- `pathType` can only be set to `Prefix`. Paths with other values are ignored.
- Every rule requires an `path` provided.
- Multiple ingresses of the `aether` type is not supported, and is considered undefined behaviour.
- Route matching is missing prioritization.
- The connections to the service are distributed completely randomly, which is suboptimal.
- It does not have most additional features you might expect from other providers, like caching.
- Wildcards in TLS secrets is not supported.

## Development

The easiest way to develop this provider is running in an actual Kubernetes cluster.
Follow the normal installation procedure, and then install `telepresence` (perferably v1).

Now you can swap out the ingress with the following command:

```bash
telepresence --swap-deployment aether-ingress --namespace aether --expose 8000
```

This forwards all requests to the ingress provider to a local server running on port 8000.
Now you can run the provider locally with:

```
RUST_LOG=debug PROOT_NO_SECCOMP=1 proot -b $TELEPRESENCE_ROOT/var/run/secrets/:/var/run/secrets cargo run
```

Just load the external loadbalancer IP in your browser, and see the result!
