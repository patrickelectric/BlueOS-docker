<template>
  <div ref="container" class="digital-twin-container">
    <div v-if="!model_loaded && !load_error" class="d-flex justify-center align-center fill-height">
      <v-progress-circular indeterminate color="primary" size="40" />
    </div>
    <div v-if="load_error" class="d-flex flex-column align-center justify-center fill-height">
      <v-icon size="60" color="grey">
        mdi-cube-off-outline
      </v-icon>
      <p class="text-body-2 grey--text mt-2">
        3D model unavailable
      </p>
    </div>
  </div>
</template>

<script lang="ts">
import * as THREE from 'three'
import { DRACOLoader } from 'three/examples/jsm/loaders/DRACOLoader'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader'
import Vue from 'vue'

import { checkModelOverrides } from '@/components/vehiclesetup/viewers/modelHelper'
import autopilot_data from '@/store/autopilot'

const dracoFiles = import.meta.glob('/node_modules/three/examples/jsm/libs/draco/*', { eager: true, as: 'url' })

export default Vue.extend({
  name: 'DigitalTwin',
  props: {
    roll: {
      type: Number,
      default: 0,
    },
    pitch: {
      type: Number,
      default: 0,
    },
    yaw: {
      type: Number,
      default: 0,
    },
  },
  data() {
    return {
      model_loaded: false,
      load_error: false,
      renderer: null as THREE.WebGLRenderer | null,
      scene: null as THREE.Scene | null,
      camera: null as THREE.PerspectiveCamera | null,
      vehicleGroup: null as THREE.Group | null,
      renderRequested: false,
      resizeObserver: null as ResizeObserver | null,
    }
  },
  computed: {
    vehicle_model(): string {
      return autopilot_data.vehicle_model
    },
  },
  watch: {
    roll() { this.updateOrientation() },
    pitch() { this.updateOrientation() },
    yaw() { this.updateOrientation() },
    vehicle_model() { this.loadModel() },
  },
  mounted() {
    this.initThreeJS()
    this.loadModel()
    this.resizeObserver = new ResizeObserver(() => this.handleResize())
    this.resizeObserver.observe(this.$refs.container as HTMLElement)
  },
  beforeDestroy() {
    this.resizeObserver?.disconnect()
    this.cleanup()
  },
  methods: {
    initThreeJS() {
      const container = this.$refs.container as HTMLElement
      if (!container) return

      this.scene = new THREE.Scene()
      this.scene.background = new THREE.Color(0xffffff)

      this.camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.01, 100)
      this.camera.position.set(0.35, 0.25, 0.35)
      this.camera.lookAt(0, 0, 0)

      this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true })
      this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
      this.renderer.setSize(container.clientWidth, container.clientHeight)
      container.appendChild(this.renderer.domElement)

      const ambient = new THREE.AmbientLight(0xffffff, 3.0)
      this.scene.add(ambient)

      const directional = new THREE.DirectionalLight(0xffffff, 2.0)
      directional.position.set(1, 1, 1)
      this.scene.add(directional)

      const fill = new THREE.DirectionalLight(0xffffff, 0.5)
      fill.position.set(-1, 0.5, -1)
      this.scene.add(fill)

      this.vehicleGroup = new THREE.Group()
      this.scene.add(this.vehicleGroup)
    },
    async loadModel() {
      if (!this.scene || !this.vehicleGroup) return

      // Clear existing model
      while (this.vehicleGroup.children.length > 0) {
        this.vehicleGroup.remove(this.vehicleGroup.children[0])
      }
      this.model_loaded = false
      this.load_error = false

      const overridePath = await checkModelOverrides()
      const modelPath = overridePath || this.vehicle_model
      if (!modelPath) {
        this.load_error = true
        return
      }

      const dracoLoader = new DRACOLoader()
      const dracoWasmFile = Object.keys(dracoFiles).find((key) => key.includes('draco_decoder.wasm'))
      if (dracoWasmFile) {
        const fileUrl = dracoFiles[dracoWasmFile] as string
        const basePath = fileUrl.replace(/[^/]*$/, '')
        dracoLoader.setDecoderPath(basePath)
      }

      const loader = new GLTFLoader()
      loader.setDRACOLoader(dracoLoader)

      loader.load(
        modelPath,
        (gltf) => {
          if (!this.vehicleGroup) return
          this.vehicleGroup.add(gltf.scene)
          this.model_loaded = true
          this.updateOrientation()
          this.requestRender()
        },
        undefined,
        () => {
          this.load_error = true
        },
      )
    },
    updateOrientation() {
      if (!this.vehicleGroup) return

      // ArduPilot NED to Three.js Y-up: swap Y and Z, negate pitch and yaw
      const euler = new THREE.Euler(this.roll, -this.yaw, -this.pitch, 'YZX')
      // Pre-rotate 90° around X to convert from NED (Z-down) to Three.js (Y-up)
      const nedToYUp = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), -Math.PI / 2)
      const attitudeQuat = new THREE.Quaternion().setFromEuler(euler)
      this.vehicleGroup.quaternion.copy(nedToYUp).multiply(attitudeQuat)

      this.requestRender()
    },
    requestRender() {
      if (this.renderRequested) return
      this.renderRequested = true
      requestAnimationFrame(() => {
        this.renderRequested = false
        this.render()
      })
    },
    render() {
      if (!this.renderer || !this.scene || !this.camera) return
      this.renderer.render(this.scene, this.camera)
    },
    handleResize() {
      const container = this.$refs.container as HTMLElement
      if (!container || !this.camera || !this.renderer) return

      const width = container.clientWidth
      const height = container.clientHeight
      if (width === 0 || height === 0) return

      this.camera.aspect = width / height
      this.camera.updateProjectionMatrix()
      this.renderer.setSize(width, height)
      this.requestRender()
    },
    cleanup() {
      if (this.renderer) {
        this.renderer.dispose()
        this.renderer.forceContextLoss()
        const container = this.$refs.container as HTMLElement
        if (container && this.renderer.domElement.parentElement === container) {
          container.removeChild(this.renderer.domElement)
        }
        this.renderer = null
      }
      if (this.scene) {
        this.scene.traverse((obj) => {
          if (obj instanceof THREE.Mesh) {
            obj.geometry?.dispose()
            if (Array.isArray(obj.material)) {
              obj.material.forEach((m: THREE.Material) => m.dispose())
            } else {
              obj.material?.dispose()
            }
          }
        })
        this.scene = null
      }
      this.camera = null
      this.vehicleGroup = null
    },
  },
})
</script>

<style scoped>
.digital-twin-container {
  width: 100%;
  height: 100%;
  min-height: 100px;
  position: relative;
}

.digital-twin-container canvas {
  display: block;
  width: 100%;
  height: 100%;
}
</style>
