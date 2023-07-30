export const FEATURES: { [key: string]: { description?: string } } = {
  "android.hardware.audio.low_latency": {
    description:
      "The app uses the device's low-latency audio pipeline, which reduces lag and delays when processing sound input or output.",
  },
  "android.hardware.audio.output": {
    description:
      "The app transmits sound using the device's speakers, audio jack, Bluetooth streaming capabilities, or a similar mechanism.",
  },
  "android.hardware.audio.pro": {
    description:
      "The app uses the device's high-end audio functionality and performance capabilities.",
  },
  "android.hardware.microphone": {
    description: "The app records audio using the device's microphone.",
  },
  "android.hardware.bluetooth": {
    description:
      "The app uses the device's Bluetooth features, usually to communicate with other Bluetooth-enabled devices.",
  },
  "android.hardware.bluetooth_le": {
    description:
      "The app uses the device's Bluetooth Low Energy radio features.",
  },
  "android.hardware.camera": {
    description:
      "The app uses the device's back-facing camera. Devices with only a front-facing camera do not list this feature, so use the android.hardware.camera.any feature instead if your app can communicate with any camera, regardless of which direction the camera faces.",
  },
  "android.hardware.camera.any": {
    description:
      "The app uses one of the device's cameras, or an external camera that the user connects to the device. Use this value instead of android.hardware.camera if your app does not require the camera to be a back-facing one.",
  },
  "android.hardware.camera.autofocus": {
    description:
      "The app uses the autofocus feature that the device's camera supports.",
  },
  "android.hardware.camera.capability.manual_post_processing": {
    description:
      "The app uses the MANUAL_POST_PROCESSING feature that the device's camera supports.",
  },
  "android.hardware.camera.capability.manual_sensor": {
    description:
      "The app uses the MANUAL_SENSOR feature that the device's camera supports.",
  },
  "android.hardware.camera.capability.raw": {
    description:
      "The app uses the RAW feature that the device's camera supports.",
  },
  "android.hardware.camera.external": {
    description:
      "The app communicates with an external camera that the user connects to the device. This feature does not guarantee, however, that the external camera is available for your app to use.",
  },
  "android.hardware.camera.flash": {
    description:
      "The app uses the flash feature that the device's camera supports.",
  },
  "android.hardware.camera.front": {
    description: "The app uses the device's front-facing camera.",
  },
  "android.hardware.camera.level.full": {
    description:
      "The app uses the FULL-level image-capturing support that at least one of the device's cameras provides. Cameras with FULL support provide burst-capture capabilities, per-frame control, and manual post-processing control.",
  },
  "android.hardware.type.automotive": {
    description:
      "The app is designed to show its UI on a set of screens inside a vehicle. The user interacts with the app using hard buttons, touch, rotary controllers, and mouse-like interfaces. The vehicle's screens usually appear in the center console or the instrument cluster of a vehicle. These screens usually have limited size and resolution.",
  },
  "android.hardware.type.television": {
    description:
      'The app is designed to show its UI on a television. This feature defines "television" to be a typical living room television experience: displayed on a big screen, where the user is sitting far away and the dominant form of input is something like a d-pad, and generally not using a mouse, pointer, or touch device.',
  },
  "android.hardware.type.watch": {
    description:
      "The app is designed to show its UI on a watch. A watch is worn on the body, such as on the wrist. The user is very close to the device while interacting with it.",
  },
  "android.hardware.fingerprint": {
    description:
      "The app reads fingerprints using the device's biometric hardware.",
  },
  "android.hardware.gamepad": {
    description:
      "The app captures game controller input, either from the device itself or from a connected gamepad.",
  },
  "android.hardware.consumerir": {
    description:
      "The app uses the device's infrared (IR) capabilities, usually to communicate with other consumer IR devices.",
  },
  "android.hardware.location": {
    description:
      "The app uses one or more features on the device for determining location, such as GPS location, network location, or cell location.",
  },
  "android.hardware.location.gps": {
    description:
      "The app uses precise location coordinates obtained from a Global Positioning System (GPS) receiver on the device.",
  },
  "android.hardware.location.network": {
    description:
      "The app uses coarse location coordinates obtained from a network-based geolocation system supported on the device.",
  },
  "android.hardware.nfc": {
    description:
      "The app uses the device's Near-Field Communication (NFC) radio features.",
  },
  "android.hardware.nfc.hce": {
    description:
      "The app uses NFC card emulation that is hosted on the device.",
  },
  "android.hardware.opengles.aep": {
    description:
      "The app uses the OpenGL ES Android Extension Packthat is installed on the device.",
  },
  "android.hardware.sensor.accelerometer": {
    description:
      "The app uses motion readings from the device's accelerometer to detect the device's current orientation. For example, an app could use accelerometer readings to determine when to switch between portrait and landscape orientations.",
  },
  "android.hardware.sensor.ambient_temperature": {
    description:
      "The app uses the device's ambient (environmental) temperature sensor. For example, a weather app could report indoor or outdoor temperature.",
  },
  "android.hardware.sensor.barometer": {
    description:
      "The app uses the device's barometer. For example, a weather app could report air pressure.",
  },
  "android.hardware.sensor.compass": {
    description:
      "The app uses the device's magnetometer (compass). For example, a navigation app could show the current direction a user faces.",
  },
  "android.hardware.sensor.gyroscope": {
    description:
      "The app uses the device's gyroscope to detect rotation and twist, creating a six-axis orientation system. By using this sensor, an app can detect more smoothly whether it needs to switch between portrait and landscape orientations.",
  },
  "android.hardware.sensor.hifi_sensors": {
    description:
      "The app uses the device's high fidelity (Hi-Fi) sensors. For example, a gaming app could detect the user's high-precision movements.",
  },
  "android.hardware.sensor.heartrate": {
    description:
      "The app uses the device's heart rate monitor. For example, a fitness app could report trends in a user's heart rate over time.",
  },
  "android.hardware.sensor.heartrate.ecg": {
    description:
      "The app uses the device's elcardiogram (ECG) heart rate sensor. For example, a fitness app could report more detailed information about a user's heart rate.",
  },
  "android.hardware.sensor.light": {
    description:
      "The app uses the device's light sensor. For example, an app could display one of two different color schemes based on the ambient lighting conditions.",
  },
  "android.hardware.sensor.proximity": {
    description:
      "The app uses the device's proximity sensor. For example, a telephony app could turn off the device's screen when the app detects that the user is holding the device close to their body.",
  },
  "android.hardware.sensor.relative_humidity": {
    description:
      "The app uses the device's relative humidity sensor. For example, a weather app could use the humidity to calculate and report the current dewpoint.",
  },
  "android.hardware.sensor.stepcounter": {
    description:
      "The app uses the device's step counter. For example, a fitness app could report the number of steps a user needs to take to achieve their daily step count goal.",
  },
  "android.hardware.sensor.stepdetector": {
    description:
      "The app uses the device's step detector. For example, a fitness app could use the time interval between steps to infer the type of exercise that the user is doing.",
  },
  "android.hardware.screen.landscape": {
    description:
      "The app requires the device to use the landscape orientation. If your app supports both orientations, then you don't need to declare either feature.",
  },
  "android.hardware.screen.portrait": {
    description:
      "The app requires the device to use the portrait orientation. If your app supports both orientations, then you don't need to declare either feature.",
  },
  "android.hardware.telephony": {
    description:
      "The app uses the device's telephony features, such as telephony radio with data communication services.",
  },
  "android.hardware.telephony.cdma": {
    description:
      "The app uses the Code Division Multiple Access (CDMA) telephony radio system.",
  },
  "android.hardware.faketouch": {
    description:
      "The app uses basic touch interaction events, such as tapping and dragging.",
  },
  "android.hardware.faketouch.multitouch.distinct": {
    description:
      'The app tracks two or more distinct "fingers" on a fake touch interface. This is a superset of the android.hardware.faketouch feature. When declared as required, this feature indicates that the app is compatible with a device only if that device emulates distinct tracking of two or more fingers or has an actual touchscreen.',
  },
  "android.hardware.touchscreen.multitouch": {
    description:
      "The app uses the device's basic two-point multitouch capabilities, such as for pinch gestures, but the app does not need to track touches independently. This is a superset of the android.hardware.touchscreen feature.",
  },
  "android.hardware.touchscreen.multitouch.distinct": {
    description:
      "The app uses the device's advanced multitouch capabilities for tracking two or more points independently. This feature is a superset of the android.hardware.touchscreen.multitouch feature.",
  },
  "android.hardware.touchscreen.multitouch.jazzhand": {
    description:
      "The app uses the device's advanced multitouch capabilities for tracking five or more points independently. This feature is a superset of the android.hardware.touchscreen.multitouch feature.",
  },
  "android.hardware.usb.accessory": {
    description: "The app behaves as the USB device and connects to USB hosts.",
  },
  "android.hardware.usb.host": {
    description:
      "The app uses the USB accessories that are connected to the device. The device serves as the USB host.",
  },
  "android.hardware.vulkan.compute": {
    description:
      "The app uses Vulkan compute features. This feature indicates that the app requires the hardware accelerated Vulkan implementation.",
  },
  "android.hardware.vulkan.level": {
    description:
      "The app uses Vulkan level features. This feature indicates that the app requires the hardware accelerated Vulkan implementation.",
  },
  "android.hardware.vulkan.version": {
    description:
      "The app uses Vulkan. This feature indicates that the app requires the hardware accelerated Vulkan implementation.",
  },
  "android.hardware.wifi": {
    description:
      "The app uses 802.11 networking (Wi-Fi) features on the device.",
  },
  "android.hardware.wifi.direct": {
    description:
      "The app uses the Wi-Fi Direct networking features on the device.",
  },
  "android.software.sip": {
    description:
      "The app uses Session Initiation Protocol (SIP) services. By using SIP, the app can support internet telephony operations, such as video conferencing and instant messaging.",
  },
  "android.software.sip.voip": {
    description:
      "The app uses SIP-based Voice Over Internet Protocol (VoIP) services. By using VoIP, the app can support real-time internet telephony operations, such as two-way video conferencing.",
  },
  "android.software.webview": {
    description: "The app displays content from the internet.",
  },
  "android.software.input_methods": {
    description:
      "The app uses a new input method, which the developer defines in an InputMethodService.",
  },
  "android.software.backup": {
    description:
      "The app includes logic to handle a backup and restore operation.",
  },
  "android.software.device_admin": {
    description:
      "The app uses device administrators to enforce a device policy.",
  },
  "android.software.managed_users": {
    description: "The app supports secondary users and managed profiles.",
  },
  "android.software.securely_removes_users": {
    description:
      "The app can permanently remove users and their associated data.",
  },
  "android.software.verified_boot": {
    description:
      "The app includes logic to handle results from the device's verified boot feature, which detects whether the device's configuration changes during a restart operation.",
  },
  "android.software.midi": {
    description:
      "The app connects to musical instruments or outputs sound using the Musical Instrument Digital Interface (MIDI) protocol.",
  },
  "android.software.print": {
    description:
      "The app includes commands for printing documents displayed on the device.",
  },
  "android.software.leanback": {
    description: "The app is designed to run on Android TV devices.",
  },
  "android.software.live_tv": {
    description: "The app streams live television programs.",
  },
  "android.software.app_widgets": {
    description:
      "The app uses or provides App Widgets and should be installed only on devices that include a Home screen or similar location where users can embed App Widgets.",
  },
  "android.software.home_screen": {
    description:
      "The app behaves as a replacement to the device's Home screen.",
  },
  "android.software.live_wallpaper": {
    description: "The app uses or provides wallpapers that include animation.",
  },
};
