terraform {
  required_providers {
    docker = {
      source  = "kreuzwerker/docker"
      version = "3.0.2"
    }
  }
}

provider "docker" {}

variable "ipfs" {
  type = object({
    gateway_port = number
    data_path    = string
  })
}

variable "qdrant" {
  type = object({
    grpc      = number
    http      = number
    data_path = string
  })
}

variable "archiver_backend" {
  type = object({
    http       = number
    openai_key = string
    data_path  = string
  })
}

resource "docker_network" "archiver" {
  name = "archiver"
}