resource "docker_image" "qdrant" {
  name = "qdrant/qdrant"
}

resource "docker_container" "qdrant" {
  name  = "qdrant"
  image = docker_image.qdrant.image_id

  networks_advanced {
    name = docker_network.archiver.id
  }

  ports {
    internal = 6333
    external = var.qdrant.http
    ip       = "127.0.0.1"
  }

  ports {
    internal = 6334
    external = var.qdrant.grpc
    ip       = "127.0.0.1"
  }

  volumes {
    container_path = "/qdrant/storage"
    host_path      = abspath(var.qdrant.data_path)
  }
}

