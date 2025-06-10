<template>
  <v-container fluid>
    <v-row>
      <v-col
        sm="4"
      >
        <v-sheet
          rounded="lg"
          min-height="268"
        >
          <v-card
            class="mx-auto height-limited"
            max-height="700px"
          >
            <v-card-title>
              <v-text-field
                v-model="topic_filter"
                :label="`Search Topics (${filtered_topics.length})`"
                clearable
                prepend-inner-icon="mdi-magnify"
                single-line
                hide-details
                class="mt-0 pt-0"
              />
            </v-card-title>
            <v-divider />
            <v-list shaped>
              <v-list-item-group
                v-model="selected_topic"
              >
                <template v-for="(item, i) in filtered_topics">
                  <v-list-item
                    :key="i"
                    :value="item"
                    active-class="deep-purple--text text--accent-4"
                  >
                    <template #default="{ active }">
                      <v-list-item-content>
                        <v-list-item-title>
                          {{ item }}
                        </v-list-item-title>
                      </v-list-item-content>

                      <v-list-item-action>
                        <v-radio
                          :input-value="active"
                          color="deep-purple accent-4"
                        />
                      </v-list-item-action>
                    </template>
                  </v-list-item>
                </template>
              </v-list-item-group>
            </v-list>
          </v-card>
        </v-sheet>
      </v-col>

      <v-col
        sm="8"
      >
        <v-card
          outlined
          width="100%"
          height="700px"
          class="d-flex flex-column"
        >
          <template
            v-if="selected_topic"
          >
            <v-card-title>
              {{ selected_topic }}
              <v-chip
                v-tooltip="'Topic liveliness status'"
                :color="topic_liveliness[selected_topic] === undefined ? 'grey'
                  : (topic_liveliness[selected_topic] ? 'green' : 'red')"
                class="ml-2"
              >
                {{ topic_liveliness[selected_topic] === undefined ? 'Unknown'
                  : (topic_liveliness[selected_topic] ? 'Alive' : 'Dead') }}
              </v-chip>
              <v-chip
                v-tooltip="'Topic type'"
                color="blue"
                class="ml-2"
              >
                {{ topic_types[selected_topic] || 'Unknown' }}
              </v-chip>
              <v-chip
                v-tooltip="'Topic message serialization type'"
                color="purple"
                class="ml-2"
              >
                {{ topic_message_types[selected_topic] || 'Unknown' }}
              </v-chip>
            </v-card-title>

            <v-card-text class="flex-grow-1 overflow-auto">
              <template v-if="isVideoTopic">
                <canvas ref="videoCanvas" width="640" height="480" />
              </template>
              <template v-else>
                <pre>{{ formatMessage(current_message) }}</pre>
              </template>
            </v-card-text>
          </template>
          <div
            v-else
            class="select-topic d-flex align-center justify-center fill-height"
          >
            <span style="font-size: 1.5rem; font-weight: 500;">
              Select a topic to view its messages.
            </span>
          </div>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>
<script lang="ts">
import {
  Config, Sample, SampleKind, Session, Subscriber,
} from '@eclipse-zenoh/zenoh-ts'
import { FFmpeg } from '@ffmpeg/ffmpeg'
import type { LogEvent } from '@ffmpeg/ffmpeg'
import { fetchFile, toBlobURL } from '@ffmpeg/util'
import Vue from 'vue'

interface VideoFrame {
  width: number
  height: number
  close(): void
}

interface VideoDecoder {
  configure(config: { codec: string; optimizeForLatency: boolean }): Promise<void>
  decode(chunk: EncodedVideoChunk): void
  close(): void
}

interface ZenohMessage {
  topic: string
  payload: string | Uint8Array
  timestamp: Date
}

export default Vue.extend({
  name: 'ZenohInspector',
  data() {
    return {
      topics: [] as string[],
      messages: {} as { [key: string]: ZenohMessage },
      topic_liveliness: {} as { [key: string]: boolean },
      topic_types: {} as { [key: string]: string },
      topic_message_types: {} as { [key: string]: string },
      selected_topic: null as string | null,
      topic_filter: '',
      session: null as Session | null,
      subscriber: null as Subscriber | null,
      liveliness_subscriber: null as Subscriber | null,
      videoDecoder: null as VideoDecoder | null,
      ffmpeg: null as FFmpeg | null,
      videoChunks: [] as Uint8Array[],
      chunkCount: 0,
      sps: null as Uint8Array | null,
      pps: null as Uint8Array | null,
      process_video_chunk: false,
    }
  },
  computed: {
    filtered_topics(): string[] {
      try {
        return this.topics.filter(
          (name: string) => name.toLowerCase().includes(this.topic_filter.toLowerCase().trim()),
        )
      } catch {
        return this.topics
      }
    },
    current_message(): ZenohMessage | null {
      if (!this.selected_topic) return null
      return this.messages[this.selected_topic] || null
    },
    isVideoTopic(): boolean {
      return this.selected_topic?.toLowerCase().includes('video') || false
    },
  },
  watch: {
    selected_topic(newTopic: string | null) {
      if (newTopic && this.isVideoTopic) {
        this.$nextTick(async () => {
          await this.setupVideoDecoder()
        })
        return
      }

      this.cleanupVideoDecoder()
    },
  },
  async mounted() {
    await this.setupZenoh()
  },
  beforeDestroy() {
    this.disconnectZenoh()
    this.cleanupVideoDecoder()
  },
  methods: {
    async setupVideoDecoder() {
      console.log('Setting up video decoder')
      const canvas = this.$refs.videoCanvas as HTMLCanvasElement
      if (!canvas) {
        console.error('Canvas element not found')
        return
      }

      const ctx = canvas.getContext('2d')
      if (!ctx) {
        console.error('Could not get canvas context')
        return
      }

      // setup ffmpeg
      const ffmpeg = new FFmpeg()
      const baseURL = 'https://cdn.jsdelivr.net/npm/@ffmpeg/core@0.12.9/dist/esm'
      /*
      ffmpeg.on('log', ({ message: msg }: LogEvent) => {
        console.log('FFmpeg log:', msg)
      })
        */
      await ffmpeg.load({
        coreURL: await toBlobURL(`${baseURL}/ffmpeg-core.js`, 'text/javascript'),
        wasmURL: await toBlobURL(`${baseURL}/ffmpeg-core.wasm`, 'application/wasm')
      })
      console.log('FFmpeg loaded')

      // Store ffmpeg instance for later use
      this.ffmpeg = ffmpeg
    },

    async processVideoChunk(chunkData: Uint8Array) {
      const baseURL = 'https://cdn.jsdelivr.net/npm/@ffmpeg/core@0.12.9/dist/esm'
      if (!this.ffmpeg) {
        console.error('FFmpeg not initialized')
        return
      }


      try {
        // Check for SPS and PPS in the chunk
        const startCode = new Uint8Array([0x00, 0x00, 0x00, 0x01])
        let offset = 0

        // Create a copy of the chunk before storing it
        const chunkCopy = new Uint8Array(chunkData.length)
        chunkCopy.set(chunkData)
        this.videoChunks.push(chunkCopy)
        this.chunkCount++

        // Limit videoChunks array size to 200
        //console.log('Chunk count:', this.chunkCount)
        if (this.videoChunks.length > 200) {
          this.videoChunks = this.videoChunks.slice(-200)
        }

        //console.log('Chunk count:', this.chunkCount)
        let resultfinal = undefined

        if (this.process_video_chunk) {
          return
        }

        if (this.chunkCount > 50) {
          this.process_video_chunk = true
          console.log("we are in")
          // Combine chunks here
          /*
          if (!this.sps || !this.pps) {
            console.error('Missing SPS or PPS data')
            return
          }
            */

          // Calculate total length of all chunks
          const totalLength = this.videoChunks.reduce((acc, chunk) => acc + chunk.length, 0)

          // Create combined array
          const combinedChunks = new Uint8Array(totalLength)

          // Copy all chunks into the combined array
          let offset = 0
          for (const chunk of this.videoChunks) {
            combinedChunks.set(chunk, offset)
            offset += chunk.length
          }

          resultfinal = combinedChunks
        }

        if (!resultfinal) {
          this.process_video_chunk = false
          return
        }

        // Process current chunk for display
        await this.ffmpeg.writeFile('input.h264', resultfinal)
        try {
          // Calculate how long it takes to run the next command
          const start = Date.now()
          await this.ffmpeg.exec(['-sseof', '-1', '-i', 'input.h264', '-update', '1', 'frame.jpg']);
          const end = Date.now()
          const duration = end - start
          console.log(`FFmpeg command took ${duration}ms`)
        } catch (error) {
          console.error('Error processing video chunk:', error)
          // delete input.h64
          //this.ffmpeg.deleteFile('input.h264')

          // reinit ffmpeg
          this.ffmpeg.terminate()
          this.ffmpeg = new FFmpeg()
          await this.ffmpeg.load({
            coreURL: await toBlobURL(`${baseURL}/ffmpeg-core.js`, 'text/javascript'),
            wasmURL: await toBlobURL(`${baseURL}/ffmpeg-core.wasm`, 'application/wasm')
          })
          this.process_video_chunk = false
          return
        }
        const frame = await this.ffmpeg.readFile('frame.jpg')
        const blob = new Blob([frame], { type: 'image/jpeg' })
        const url = URL.createObjectURL(blob)

        // Draw the frame
        const img = new Image()
        img.onload = () => {
          const canvas = this.$refs.videoCanvas as HTMLCanvasElement
          if (canvas) {
            canvas.width = img.width
            canvas.height = img.height
            const ctx = canvas.getContext('2d')
            if (ctx) {
              ctx.drawImage(img, 0, 0)
            }
          }
          URL.revokeObjectURL(url)
        }
        img.src = url
      } catch (error) {
        console.error('Error processing video chunk:', error)
      }
      this.process_video_chunk = false
    },

    async combineAndDownloadChunks(combinedChunks: Uint8Array) {
      try {
        // Create a blob and download
        const blob = new Blob([combinedChunks], { type: 'video/h264' })
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `video_${new Date().toISOString()}.h264`
        document.body.appendChild(a)
        a.click()
        document.body.removeChild(a)
        URL.revokeObjectURL(url)

        // Reset buffer and counter
        this.videoChunks = []
        this.chunkCount = 0
      } catch (error) {
        console.error('Error combining and downloading chunks:', error)
        // Reset buffer and counter even if there's an error
        this.videoChunks = []
        this.chunkCount = 0
      }
    },

    cleanupVideoDecoder() {
      if (this.videoDecoder) {
        this.videoDecoder.close()
        this.videoDecoder = null
      }
      // Clear video chunks buffer when cleaning up
      this.videoChunks = []
      this.chunkCount = 0
    },
    formatMessage(message: ZenohMessage | null): string {
      if (!message) return 'No messages received yet'

      // Create the base message object
      const formattedMessage = {
        topic: message.topic,
        timestamp: message.timestamp.toLocaleString(),
        // eslint-disable-next-line no-nested-ternary
        liveliness: this.topic_liveliness[message.topic] === undefined ? 'Unknown'
          : this.topic_liveliness[message.topic] ? 'Alive' : 'Dead',
        topic_type: this.topic_types[message.topic] || 'Unknown',
        message_type: this.topic_message_types[message.topic] || 'Unknown',
        payload: message.payload,
      }

      // Try to parse the payload as JSON if possible
      if (typeof message.payload === 'string') {
        try {
          formattedMessage.payload = JSON.parse(message.payload)
        } catch (exception) {
          // Keep the raw payload if it's not valid JSON
        }
      }

      return JSON.stringify(formattedMessage, null, 2)
    },
    async setupZenoh() {
      try {
        const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws'
        const url = `${protocol}://${window.location.host}/zenoh-api/`
        const config = new Config(url)
        this.session = await Session.open(config)

        // Setup regular message subscriber
        this.subscriber = await this.session.declare_subscriber('**', {
          handler: async (sample: Sample) => {
            const topic = sample.keyexpr().toString()
            const payload = sample.payload()
            const message: ZenohMessage = {
              topic,
              payload: topic.toLowerCase().includes('video')
                ? payload.to_bytes()
                : payload.to_string(),
              timestamp: new Date(),
            }

            // Update messages and topics
            this.$set(this.messages, topic, message)
            if (!this.topics.includes(topic)) {
              this.topics = [...this.topics, topic].sort()
            }

            // Handle H264 video decoding
            if (
              topic === this.selected_topic
              && this.isVideoTopic
              && message.payload instanceof Uint8Array
            ) {
              try {
                await this.processVideoChunk(message.payload)
              } catch (error) {
                console.error('Error processing video chunk:', error)
              }
            }

            return Promise.resolve()
          },
        })

        // Setup liveliness subscriber
        const lv_ke = '@/**/@ros2_lv/**'

        this.liveliness_subscriber = await this.session.liveliness().declare_subscriber(lv_ke, {
          handler: (sample: Sample) => {
            // Parse the liveliness token using regex
            // eslint-disable-next-line max-len
            // https://github.com/eclipse-zenoh/zenoh-plugin-ros2dds/blob/865d3db009d0d2635826700a35483e88a077967d/zenoh-plugin-ros2dds/src/liveliness_mgt.rs#L202
            const keyexpr = sample.keyexpr().toString()
            // eslint-disable-next-line max-len
            const match = keyexpr.match(/@\/(?<zenoh_id>[^/]+)\/@ros2_lv\/(?<type>MP|MS|SS|SC|AS|AC)\/(?<ke>[^/]+)\/(?<typ>[^/]+)(?:\/(?<qos_ke>[^/]+))?/)

            if (!match) {
              return Promise.resolve()
            }

            const { type, ke, typ } = match.groups || {}
            const topic = ke.replace(/§/g, '/')
            const messageTyp = typ.replace(/§/g, '/')

            const isAlive = sample.kind() === SampleKind.PUT

            // Update liveliness state and type
            this.$set(this.topic_liveliness, topic, isAlive)
            this.$set(this.topic_types, topic, type)
            this.$set(this.topic_message_types, topic, messageTyp)

            // Add to topics if not already present
            if (!this.topics.includes(topic)) {
              this.topics = [...this.topics, topic].sort()
            }

            return Promise.resolve()
          },
          history: true, // Enable history to get initial state
        })
      } catch (error) {
        console.error('[Zenoh] Connection error:', error)
      }
    },
    async disconnectZenoh() {
      await this.session?.close()
      this.subscriber?.undeclare()
      this.liveliness_subscriber?.undeclare()

      this.session = null
      this.subscriber = null
      this.liveliness_subscriber = null
    },
  },
})
</script>
<style>
.height-limited {
  overflow-y: auto;
  max-height: 700px;
}

.select-topic {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  text-align: center;
}
</style>
