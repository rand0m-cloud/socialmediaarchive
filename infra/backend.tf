resource "docker_image" "backend" {
  name = "archiver_backend"

  build {
    context    = ".."
    dockerfile = "infra/Dockerfile"
  }

  triggers = {
    dir_sha1   = sha1(join("", [for f in fileset("${path.root}/..", "backend/src/*") : filesha1("${path.root}/../${f}")]))
    dockerfile = filesha1("Dockerfile")
  }
}

resource "docker_container" "backend" {
  name  = "archiver_backend"
  image = docker_image.backend.image_id

  networks_advanced {
    name = docker_network.archiver.name
  }

  ports {
    internal = 5003
    external = var.archiver_backend.http
  }

  env = [
    "RUST_LOG=info",
    "OPENAI_KEY=${var.archiver_backend.openai_key}",
    "QDRANT_URL=http://${docker_container.qdrant.name}:${var.qdrant.grpc}",
    "IPFS_URL=http://${docker_container.ipfs.name}:5001"
  ]

  volumes {
    container_path = "/app"
    host_path      = abspath(var.archiver_backend.data_path)
  }
}