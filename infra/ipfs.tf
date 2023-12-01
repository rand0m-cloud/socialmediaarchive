resource "docker_image" "ipfs" {
  name = "ipfs/kubo"
}

resource "docker_container" "ipfs" {
  name    = "ipfs"
  image   = docker_image.ipfs.image_id
  restart = "unless-stopped"

  networks_advanced {
    name = docker_network.archiver.id
  }

  ports {
    internal = 5001
    external = 5001
    ip       = "127.0.0.1"
  }

  ports {
    internal = 4001
    external = 4001
  }

  ports {
    internal = 4001
    external = 4001
    protocol = "udp"
  }

  ports {
    internal = 8080
    external = var.ipfs.gateway_port
  }

  volumes {
    container_path = "/data/ipfs"
    host_path      = abspath(var.ipfs.data_path)
  }
}
