# Aether

Simple and fast Ingress Provider and TLS termination for kubernetes.
**Currently not production-ready.**

This provider autodiscovers certificates to be used with TLS based on the configuration, rather than having to provide it in the ingress configuration.
It can be used in combination with [Cert Manager](https://cert-manager.io/) or simulair to automatically generate and serve certificates.

## Configuration

### Secret discovery

Todo:

- Tag at the ingress.
- Same namespace as the ingress.

## Limitations

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
