package metadata

base: components: sources: demo_logs: configuration: {
	count: {
		description: """
			The total number of lines to output.

			By default, the source continuously prints logs (infinitely).
			"""
		required: false
		type: uint: default: 9223372036854775807
	}
	decoding: {
		description: "Configures how events are decoded from raw bytes."
		required:    false
		type: object: {
			default: codec: "bytes"
			options: codec: {
				required: true
				type: string: enum: {
					bytes: "Uses the raw bytes as-is."
					gelf: """
						Decodes the raw bytes as a [GELF][gelf] message.

						[gelf]: https://docs.graylog.org/docs/gelf
						"""
					json: """
						Decodes the raw bytes as [JSON][json].

						[json]: https://www.json.org/
						"""
					native: """
						Decodes the raw bytes as Vector’s [native Protocol Buffers format][vector_native_protobuf] ([EXPERIMENTAL][experimental]).

						[vector_native_protobuf]: https://github.com/vectordotdev/vector/blob/master/lib/vector-core/proto/event.proto
						[experimental]: https://vector.dev/highlights/2022-03-31-native-event-codecs
						"""
					native_json: """
						Decodes the raw bytes as Vector’s [native JSON format][vector_native_json] ([EXPERIMENTAL][experimental]).

						[vector_native_json]: https://github.com/vectordotdev/vector/blob/master/lib/codecs/tests/data/native_encoding/schema.cue
						[experimental]: https://vector.dev/highlights/2022-03-31-native-event-codecs
						"""
					syslog: """
						Decodes the raw bytes as a Syslog message.

						Will decode either as the [RFC 3164][rfc3164]-style format ("old" style) or the more modern
						[RFC 5424][rfc5424]-style format ("new" style, includes structured data).

						[rfc3164]: https://www.ietf.org/rfc/rfc3164.txt
						[rfc5424]: https://www.ietf.org/rfc/rfc5424.txt
						"""
				}
			}
		}
	}
	format: {
		required: false
		type: string: {
			default: "json"
			enum: {
				apache_common: "Randomly generated logs in [Apache common](\\(urls.apache_common)) format."
				apache_error:  "Randomly generated logs in [Apache error](\\(urls.apache_error)) format."
				bsd_syslog:    "Randomly generated logs in Syslog format ([RFC 3164](\\(urls.syslog_3164)))."
				json:          "Randomly generated HTTP server logs in [JSON](\\(urls.json)) format."
				shuffle:       "Lines are chosen at random from the list specified using `lines`."
				syslog:        "Randomly generated logs in Syslog format ([RFC 5424](\\(urls.syslog_5424)))."
			}
		}
	}
	framing: {
		description: """
			Framing configuration.

			Framing deals with how events are separated when encoded in a raw byte form, where each event is
			a "frame" that must be prefixed, or delimited, in a way that marks where an event begins and
			ends within the byte stream.
			"""
		required: false
		type: object: {
			default: method: "bytes"
			options: {
				character_delimited: {
					description:   "Options for the character delimited decoder."
					relevant_when: "method = \"character_delimited\""
					required:      true
					type: object: options: {
						delimiter: {
							description: "The character that delimits byte sequences."
							required:    true
							type: uint: {}
						}
						max_length: {
							description: """
																The maximum length of the byte buffer.

																This length does *not* include the trailing delimiter.
																"""
							required: false
							type: uint: {}
						}
					}
				}
				method: {
					required: true
					type: string: enum: {
						bytes:               "Byte frames are passed through as-is according to the underlying I/O boundaries (e.g. split between messages or stream segments)."
						character_delimited: "Byte frames which are delimited by a chosen character."
						length_delimited:    "Byte frames which are prefixed by an unsigned big-endian 32-bit integer indicating the length."
						newline_delimited:   "Byte frames which are delimited by a newline character."
						octet_counting: """
															Byte frames according to the [octet counting][octet_counting] format.

															[octet_counting]: https://tools.ietf.org/html/rfc6587#section-3.4.1
															"""
					}
				}
				newline_delimited: {
					description:   "Options for the newline delimited decoder."
					relevant_when: "method = \"newline_delimited\""
					required:      false
					type: object: options: max_length: {
						description: """
																The maximum length of the byte buffer.

																This length does *not* include the trailing delimiter.
																"""
						required: false
						type: uint: {}
					}
				}
				octet_counting: {
					description:   "Options for the octet counting decoder."
					relevant_when: "method = \"octet_counting\""
					required:      false
					type: object: options: max_length: {
						description: "The maximum length of the byte buffer."
						required:    false
						type: uint: {}
					}
				}
			}
		}
	}
	interval: {
		description: """
			The amount of time, in seconds, to pause between each batch of output lines.

			The default is one batch per second. In order to remove the delay and output batches as quickly as possible, set
			`interval` to `0.0`.
			"""
		required: false
		type: float: default: 1.0
	}
	lines: {
		description:   "The list of lines to output."
		relevant_when: "format = \"shuffle\""
		required:      true
		type: array: items: type: string: syntax: "literal"
	}
	sequence: {
		description:   "If `true`, each output line starts with an increasing sequence number, beginning with 0."
		relevant_when: "format = \"shuffle\""
		required:      false
		type: bool: default: false
	}
}
