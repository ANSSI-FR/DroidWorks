export const PERMISSIONS: { [key: string]: { description?: string } } = {
  "android.permission.ACCEPT_HANDOVER": {
    //Added in API level 28
    description:
      "Allows a calling app to continue a call which was started in another app. An example is a video calling app that wants to continue a voice call on the user's mobile network. \
When the handover of a call from one app to another takes place, there are two devices which are involved in the handover; the initiating and receiving devices. The initiating device is where the request to handover the call was started, and the receiving device is where the handover request is confirmed by the other party. \
This permission protects access to the TelecomManager.acceptHandover(Uri, int, PhoneAccountHandle) which the receiving side of the handover uses to accept a handover.",
    //Protection level: dangerous
  },
  "android.permission.ACCESS_BACKGROUND_LOCATION": {
    //Added in API level 29
    description:
      "Allows an app to access location in the background. If you're requesting this permission, you must also request either ACCESS_COARSE_LOCATION or ACCESS_FINE_LOCATION. Requesting this permission by itself doesn't give you location access.",
    //Protection level: dangerous
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.ACCESS_BLOBS_ACROSS_USERS": {
    //Added in API level 31
    description: "Allows an application to access data blobs across users.",
  },
  "android.permission.ACCESS_CHECKIN_PROPERTIES": {
    //Added in API level 1
    description:
      'Allows read/write access to the "properties" table in the checkin database, to change values that get uploaded.',
    //Not for use by third-party applications.
  },
  "android.permission.ACCESS_COARSE_LOCATION": {
    //Added in API level 1
    description:
      "Allows an app to access approximate location. Alternatively, you might want ACCESS_FINE_LOCATION.",
    //Protection level: dangerous
  },
  "android.permission.ACCESS_FINE_LOCATION": {
    //Added in API level 1
    description:
      "Allows an app to access precise location. Alternatively, you might want ACCESS_COARSE_LOCATION.",
    //Protection level: dangerous
  },
  "android.permission.ACCESS_LOCATION_EXTRA_COMMANDS": {
    //Added in API level 1
    description:
      "Allows an application to access extra location provider commands.",
    //Protection level: normal
  },
  "android.permission.ACCESS_MEDIA_LOCATION": {
    //Added in API level 29
    description:
      "Allows an application to access any geographic locations persisted in the user's shared collection.",
    //Protection level: dangerous
  },
  "android.permission.ACCESS_NETWORK_STATE": {
    //Added in API level 1
    description: "Allows applications to access information about networks.",
    //Protection level: normal
  },
  "android.permission.ACCESS_NOTIFICATION_POLICY": {
    //Added in API level 23
    description:
      "Marker permission for applications that wish to access notification policy. This permission is not supported on managed profiles.",
    //Protection level: normal
  },
  "android.permission.ACCESS_WIFI_STATE": {
    //Added in API level 1
    description:
      "Allows applications to access information about Wi-Fi networks.",
    //Protection level: normal
  },
  "android.permission.ACCOUNT_MANAGER": {
    //Added in API level 5
    description: "Allows applications to call into AccountAuthenticators.",
    //Not for use by third-party applications.
  },
  "android.permission.ACTIVITY_RECOGNITION": {
    //Added in API level 29
    description: "Allows an application to recognize physical activity.",
    //Protection level: dangerous
  },
  "android.permission.ADD_VOICEMAIL": {
    //Added in API level 14
    description: "Allows an application to add voicemails into the system.",
    //Protection level: dangerous
  },
  "android.permission.ANSWER_PHONE_CALLS": {
    //Added in API level 26
    description: "Allows the app to answer an incoming phone call.",
    //Protection level: dangerous
  },
  "android.permission.BATTERY_STATS": {
    //Added in API level 1
    description: "Allows an application to collect battery statistics",
    //Protection level: signature|privileged|development
  },
  "android.permission.BIND_ACCESSIBILITY_SERVICE": {
    //Added in API level 16
    description:
      "Must be required by an AccessibilityService, to ensure that only the system can bind to it.",
    //Protection level: signature
  },
  "android.permission.BIND_APPWIDGET": {
    //Added in API level 3
    description:
      "Allows an application to tell the AppWidget service which application can access AppWidget's data. The normal user flow is that a user picks an AppWidget to go into a particular host, thereby giving that host application access to the private data from the AppWidget app. An application that has this permission should honor that contract.",
    //Not for use by third-party applications.
  },
  "android.permission.BIND_AUTOFILL_SERVICE": {
    //Added in API level 26
    description:
      "Must be required by a AutofillService, to ensure that only the system can bind to it.",
    //Protection level: signature
  },
  "android.permission.BIND_CALL_REDIRECTION_SERVICE": {
    //Added in API level 29
    description:
      "Must be required by a CallRedirectionService, to ensure that only the system can bind to it.",
    //Protection level: signature|privileged
  },
  "android.permission.BIND_CARRIER_MESSAGING_CLIENT_SERVICE": {
    //Added in API level 29
    description:
      "A subclass of CarrierMessagingClientService must be protected with this permission.",
    //Protection level: signature
  },
  "android.permission.BIND_CARRIER_MESSAGING_SERVICE": {
    //Added in API level 22
    //Deprecated in API level 23
    //
    //public static final String BIND_CARRIER_MESSAGING_SERVICE
    //
    //This constant was deprecated in API level 23.
    //Use BIND_CARRIER_SERVICES instead
  },
  "android.permission.BIND_CARRIER_SERVICES": {
    //Added in API level 23
    description:
      "The system process that is allowed to bind to services in carrier apps will have this permission. Carrier apps should use this permission to protect their services that only the system is allowed to bind to.",
    //Protection level: signature|privileged
  },
  "android.permission.BIND_CHOOSER_TARGET_SERVICE": {
    //Added in API level 23
    //Deprecated in API level 30
    //
    //public static final String BIND_CHOOSER_TARGET_SERVICE
    //
    //This constant was deprecated in API level 30.
    //For publishing direct share targets, please follow the instructions in https://developer.android.com/training/sharing/receive.html#providing-direct-share-targets instead.
    //
    //Must be required by a ChooserTargetService, to ensure that only the system can bind to it.
    //
    //Protection level: signature
  },
  "android.permission.BIND_COMPANION_DEVICE_SERVICE": {
    //Added in API level 31
    description:
      "Must be required by any CompanionDeviceServices to ensure that only the system can bind to it.",
  },
  "android.permission.BIND_CONDITION_PROVIDER_SERVICE": {
    //Added in API level 24
    description:
      "Must be required by a ConditionProviderService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_CONTROLS": {
    //Added in API level 30
    //
    //public static final String BIND_CONTROLS
    //
    description: "Allows SystemUI to request third party controls.",
    //
    //Should only be requested by the System and required by ControlsProviderService declarations.
  },
  "android.permission.BIND_DEVICE_ADMIN": {
    //Added in API level 8
    //
    //public static final String BIND_DEVICE_ADMIN
    //
    description:
      "Must be required by device administration receiver, to ensure that only the system can interact with it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_DREAM_SERVICE": {
    //Added in API level 21
    //
    //public static final String BIND_DREAM_SERVICE
    //
    description:
      "Must be required by an DreamService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_INCALL_SERVICE": {
    //Added in API level 23
    //
    //public static final String BIND_INCALL_SERVICE
    //
    description:
      "Must be required by a InCallService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BIND_INPUT_METHOD": {
    //Added in API level 3
    //
    //public static final String BIND_INPUT_METHOD
    //
    description:
      "Must be required by an InputMethodService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_MIDI_DEVICE_SERVICE": {
    //Added in API level 23
    //
    //public static final String BIND_MIDI_DEVICE_SERVICE
    //
    description:
      "Must be required by an MidiDeviceService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_NFC_SERVICE": {
    //Added in API level 19
    //
    //public static final String BIND_NFC_SERVICE
    //
    description:
      "Must be required by a HostApduService or OffHostApduService to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_NOTIFICATION_LISTENER_SERVICE": {
    //Added in API level 18
    //
    //public static final String BIND_NOTIFICATION_LISTENER_SERVICE
    //
    description:
      "Must be required by an NotificationListenerService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_PRINT_SERVICE": {
    //Added in API level 19
    //
    //public static final String BIND_PRINT_SERVICE
    //
    description:
      "Must be required by a PrintService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_QUICK_ACCESS_WALLET_SERVICE": {
    //Added in API level 30
    //
    //public static final String BIND_QUICK_ACCESS_WALLET_SERVICE
    //
    description:
      "Must be required by a QuickAccessWalletService to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_QUICK_SETTINGS_TILE": {
    //Added in API level 24
    //
    //public static final String BIND_QUICK_SETTINGS_TILE
    //
    description:
      "Allows an application to bind to third party quick settings tiles.",
    //
    //Should only be requested by the System, should be required by TileService declarations.
  },
  "android.permission.BIND_REMOTEVIEWS": {
    //Added in API level 11
    //
    //public static final String BIND_REMOTEVIEWS
    //
    description:
      "Must be required by a RemoteViewsService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BIND_SCREENING_SERVICE": {
    //Added in API level 24
    //
    //public static final String BIND_SCREENING_SERVICE
    //
    description:
      "Must be required by a CallScreeningService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BIND_TELECOM_CONNECTION_SERVICE": {
    //Added in API level 23
    //
    //public static final String BIND_TELECOM_CONNECTION_SERVICE
    //
    description:
      "Must be required by a ConnectionService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BIND_TEXT_SERVICE": {
    //Added in API level 14
    //
    //public static final String BIND_TEXT_SERVICE
    //
    description:
      "Must be required by a TextService (e.g. SpellCheckerService) to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_TV_INPUT": {
    //Added in API level 21
    //
    //public static final String BIND_TV_INPUT
    //
    description:
      "Must be required by a TvInputService to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BIND_VISUAL_VOICEMAIL_SERVICE": {
    //Added in API level 26
    //
    //public static final String BIND_VISUAL_VOICEMAIL_SERVICE
    //
    description:
      "Must be required by a link VisualVoicemailService to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BIND_VOICE_INTERACTION": {
    //Added in API level 21
    //
    //public static final String BIND_VOICE_INTERACTION
    //
    description:
      "Must be required by a VoiceInteractionService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_VPN_SERVICE": {
    //Added in API level 14
    //
    //public static final String BIND_VPN_SERVICE
    //
    description:
      "Must be required by a VpnService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_VR_LISTENER_SERVICE": {
    //Added in API level 24
    //
    //public static final String BIND_VR_LISTENER_SERVICE
    //
    description:
      "Must be required by an VrListenerService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature
  },
  "android.permission.BIND_WALLPAPER": {
    //Added in API level 8
    //
    //public static final String BIND_WALLPAPER
    //
    description:
      "Must be required by a WallpaperService, to ensure that only the system can bind to it.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.BLUETOOTH": {
    //Added in API level 1
    //
    //public static final String BLUETOOTH
    //
    description: "Allows applications to connect to paired bluetooth devices.",
    //
    //Protection level: normal
  },
  "android.permission.BLUETOOTH_ADMIN": {
    //Added in API level 1
    //
    //public static final String BLUETOOTH_ADMIN
    //
    description: "Allows applications to discover and pair bluetooth devices.",
    //
    //Protection level: normal
  },
  "android.permission.BLUETOOTH_ADVERTISE": {
    //Added in API level 31
    //
    //public static final String BLUETOOTH_ADVERTISE
    //
    description:
      "Required to be able to advertise to nearby Bluetooth devices.",
    //
    //Protection level: dangerous
  },
  "android.permission.BLUETOOTH_CONNECT": {
    //Added in API level 31
    //
    //public static final String BLUETOOTH_CONNECT
    //
    description: "Required to be able to connect to paired Bluetooth devices.",
    //
    //Protection level: dangerous
  },
  "android.permission.BLUETOOTH_PRIVILEGED": {
    //Added in API level 19
    //
    //public static final String BLUETOOTH_PRIVILEGED
    //
    description:
      "Allows applications to pair bluetooth devices without user interaction, and to allow or disallow phonebook access or message access.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.BLUETOOTH_SCAN": {
    //Added in API level 31
    //
    //public static final String BLUETOOTH_SCAN
    //
    description:
      "Required to be able to discover and pair nearby Bluetooth devices.",
    //
    //Protection level: dangerous
  },
  "android.permission.BODY_SENSORS": {
    //Added in API level 20
    //
    //public static final String BODY_SENSORS
    //
    description:
      "Allows an application to access data from sensors that the user uses to measure what is happening inside their body, such as heart rate.",
    //
    //Protection level: dangerous
  },
  "android.permission.BROADCAST_PACKAGE_REMOVED": {
    //Added in API level 1
    //
    //public static final String BROADCAST_PACKAGE_REMOVED
    //
    description:
      "Allows an application to broadcast a notification that an application package has been removed.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.BROADCAST_SMS": {
    //Added in API level 2
    //
    //public static final String BROADCAST_SMS
    //
    description:
      "Allows an application to broadcast an SMS receipt notification.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.BROADCAST_STICKY": {
    //Added in API level 1
    //
    //public static final String BROADCAST_STICKY
    //
    description:
      "Allows an application to broadcast sticky intents. These are broadcasts whose data is held by the system after being finished, so that clients can quickly retrieve that data without having to wait for the next broadcast.",
    //
    //Protection level: normal
  },
  "android.permission.BROADCAST_WAP_PUSH": {
    //Added in API level 2
    //
    //public static final String BROADCAST_WAP_PUSH
    //
    description:
      "Allows an application to broadcast a WAP PUSH receipt notification.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.CALL_COMPANION_APP": {
    //Added in API level 29
    //
    //public static final String CALL_COMPANION_APP
    //
    description:
      "Allows an app which implements the InCallService API to be eligible to be enabled as a calling companion app. This means that the Telecom framework will bind to the app's InCallService implementation when there are calls active. The app can use the InCallService API to view information about calls on the system and control these calls.",
    //
    //Protection level: normal
  },
  "android.permission.CALL_PHONE": {
    //Added in API level 1
    //
    //public static final String CALL_PHONE
    //
    description:
      "Allows an application to initiate a phone call without going through the Dialer user interface for the user to confirm the call.",
    //
    //Protection level: dangerous
  },
  "android.permission.CALL_PRIVILEGED": {
    //Added in API level 1
    //
    //public static final String CALL_PRIVILEGED
    //
    description:
      "Allows an application to call any phone number, including emergency numbers, without going through the Dialer user interface for the user to confirm the call being placed.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.CAMERA": {
    //Added in API level 1
    description: "Required to be able to access the camera device.",
    //This will automatically enforce the uses-feature manifest element for all camera features. If you do not require all camera features or can properly operate if a camera is not available, then you must modify your manifest as appropriate in order to install on devices that don't support all camera features.",
    //Protection level: dangerous
  },
  "android.permission.CAPTURE_AUDIO_OUTPUT": {
    //Added in API level 19
    //
    //public static final String CAPTURE_AUDIO_OUTPUT
    //
    description:
      "Allows an application to capture audio output. Use the CAPTURE_MEDIA_OUTPUT permission if only the USAGE_UNKNOWN), USAGE_MEDIA) or USAGE_GAME) usages are intended to be captured.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.CHANGE_COMPONENT_ENABLED_STATE": {
    //Added in API level 1
    //
    //public static final String CHANGE_COMPONENT_ENABLED_STATE
    //
    description:
      "Allows an application to change whether an application component (other than its own) is enabled or not.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.CHANGE_CONFIGURATION": {
    //Added in API level 1
    //
    //public static final String CHANGE_CONFIGURATION
    //
    description:
      "Allows an application to modify the current configuration, such as locale.",
    //
    //Protection level: signature|privileged|development
  },
  "android.permission.CHANGE_NETWORK_STATE": {
    //Added in API level 1
    //
    //public static final String CHANGE_NETWORK_STATE
    //
    description: "Allows applications to change network connectivity state.",
    //
    //Protection level: normal
  },
  "android.permission.CHANGE_WIFI_MULTICAST_STATE": {
    //Added in API level 4
    //
    //public static final String CHANGE_WIFI_MULTICAST_STATE
    //
    description: "Allows applications to enter Wi-Fi Multicast mode.",
    //
    //Protection level: normal
  },
  "android.permission.CHANGE_WIFI_STATE": {
    //Added in API level 1
    //
    //public static final String CHANGE_WIFI_STATE
    //
    description: "Allows applications to change Wi-Fi connectivity state.",
    //
    //Protection level: normal
  },
  "android.permission.CLEAR_APP_CACHE": {
    //Added in API level 1
    //
    //public static final String CLEAR_APP_CACHE
    //
    description:
      "Allows an application to clear the caches of all installed applications on the device.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.CONTROL_LOCATION_UPDATES": {
    //Added in API level 1
    //
    //public static final String CONTROL_LOCATION_UPDATES
    //
    description:
      "Allows enabling/disabling location update notifications from the radio.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.DELETE_CACHE_FILES": {
    //Added in API level 1
    //
    //public static final String DELETE_CACHE_FILES
    //
    description:
      "Old permission for deleting an app's cache files, no longer used, but signals for us to quietly ignore calls instead of throwing an exception.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.DELETE_PACKAGES": {
    //Added in API level 1
    //
    //public static final String DELETE_PACKAGES
    //
    description: "Allows an application to delete packages.",
    //
    //Not for use by third-party applications.
    //
    //Starting in Build.VERSION_CODES.N, user confirmation is requested when the application deleting the package is not the same application that installed the package.
  },
  "android.permission.DIAGNOSTIC": {
    //Added in API level 1
    //
    //public static final String DIAGNOSTIC
    //
    description: "Allows applications to RW to diagnostic resources.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.DISABLE_KEYGUARD": {
    //Added in API level 1
    //
    //public static final String DISABLE_KEYGUARD
    //
    description:
      "Allows applications to disable the keyguard if it is not secure.",
    //
    //Protection level: normal
  },
  "android.permission.DUMP": {
    //Added in API level 1
    //
    //public static final String DUMP
    //
    description:
      "Allows an application to retrieve state dump information from system services.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.EXPAND_STATUS_BAR": {
    //Added in API level 1
    //
    //public static final String EXPAND_STATUS_BAR
    //
    description: "Allows an application to expand or collapse the status bar.",
    //
    //Protection level: normal
  },
  "android.permission.FACTORY_TEST": {
    //Added in API level 1
    //
    //public static final String FACTORY_TEST
    //
    description:
      "Run as a manufacturer test application, running as the root user. Only available when the device is running in manufacturer test mode.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.FOREGROUND_SERVICE": {
    //Added in API level 28
    //
    //public static final String FOREGROUND_SERVICE
    //
    description: "Allows a regular application to use Service.startForeground.",
    //
    //Protection level: normal
  },
  "android.permission.GET_ACCOUNTS": {
    //Added in API level 1
    //
    //public static final String GET_ACCOUNTS
    //
    description:
      "Allows access to the list of accounts in the Accounts Service.",
    //
    //Note: Beginning with Android 6.0 (API level 23), if an app shares the signature of the authenticator that manages an account, it does not need "GET_ACCOUNTS" permission to read information about that account. On Android 5.1 and lower, all apps need "GET_ACCOUNTS" permission to read information about any account.
    //
    //Protection level: dangerous
  },
  "android.permission.GET_ACCOUNTS_PRIVILEGED": {
    //Added in API level 23
    //
    //public static final String GET_ACCOUNTS_PRIVILEGED
    //
    description:
      "Allows access to the list of accounts in the Accounts Service.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.GET_PACKAGE_SIZE": {
    //Added in API level 1
    //
    //public static final String GET_PACKAGE_SIZE
    //
    description:
      "Allows an application to find out the space used by any package.",
    //
    //Protection level: normal
  },
  "android.permission.GET_TASKS": {
    //Added in API level 1
    //Deprecated in API level 21
    //
    //public static final String GET_TASKS
    //
    //This constant was deprecated in API level 21.
    //No longer enforced.
  },
  "android.permission.GLOBAL_SEARCH": {
    //Added in API level 4
    //
    //public static final String GLOBAL_SEARCH
    //
    description:
      "This permission can be used on content providers to allow the global search system to access their data. Typically it used when the provider has some permissions protecting it (which global search would not be expected to hold), and added as a read-only permission to the path in the provider where global search queries are performed. This permission can not be held by regular applications; it is used by applications to protect themselves from everyone else besides global search.",
    //
    //Protection level: signature|privileged
  },
  "android.permission.HIDE_OVERLAY_WINDOWS": {
    //Added in API level 31
    //
    //public static final String HIDE_OVERLAY_WINDOWS
    //
    description:
      "Allows an app to prevent non-system-overlay windows from being drawn on top of it",
  },
  "android.permission.HIGH_SAMPLING_RATE_SENSORS": {
    //Added in API level 31
    //
    //public static final String HIGH_SAMPLING_RATE_SENSORS
    //
    description:
      "Allows an app to access sensor data with a sampling rate greater than 200 Hz.",
    //
    //Protection level: normal
  },
  "android.permission.INSTALL_LOCATION_PROVIDER": {
    //Added in API level 4
    //
    //public static final String INSTALL_LOCATION_PROVIDER
    //
    description:
      "Allows an application to install a location provider into the Location Manager.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.INSTALL_PACKAGES": {
    //Added in API level 1
    //
    //public static final String INSTALL_PACKAGES
    //
    description: "Allows an application to install packages.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.INSTALL_SHORTCUT": {
    //Added in API level 19
    //
    //public static final String INSTALL_SHORTCUT
    //
    description: "Allows an application to install a shortcut in Launcher.",
    //
    //In Android O (API level 26) and higher, the INSTALL_SHORTCUT broadcast no longer has any effect on your app because it's a private, implicit broadcast. Instead, you should create an app shortcut by using the requestPinShortcut() method from the ShortcutManager class.
    //
    //Protection level: normal
  },
  "android.permission.INSTANT_APP_FOREGROUND_SERVICE": {
    //Added in API level 26
    //
    //public static final String INSTANT_APP_FOREGROUND_SERVICE
    //
    description: "Allows an instant app to create foreground services.",
    //
    //Protection level: signature|development|instant|appop
  },
  "android.permission.INTERACT_ACROSS_PROFILES": {
    //Added in API level 30
    //
    //public static final String INTERACT_ACROSS_PROFILES
    //
    description:
      "Allows interaction across profiles in the same profile group.",
  },
  "android.permission.INTERNET": {
    //Added in API level 1
    description: "Allows applications to open network sockets.",
    //Protection level: normal
  },
  "android.permission.KILL_BACKGROUND_PROCESSES": {
    //Added in API level 8
    //
    //public static final String KILL_BACKGROUND_PROCESSES
    //
    description:
      "Allows an application to call ActivityManager.killBackgroundProcesses(String).",
    //
    //Protection level: normal
  },
  "android.permission.LOADER_USAGE_STATS": {
    //Added in API level 30
    //
    //public static final String LOADER_USAGE_STATS
    //
    //Allows a data loader to read a package's access logs. The access logs contain the set of pages referenced over time.
    //
    //Declaring the permission implies intention to use the API and the user of the device can grant permission through the Settings application.
    //
    //Protection level: signature|privileged|appop
    //
    //A data loader has to be the one which provides data to install an app.
    //
    //A data loader has to have both permission:LOADER_USAGE_STATS AND appop:LOADER_USAGE_STATS allowed to be able to access the read logs.
  },
  "android.permission.LOCATION_HARDWARE": {
    //Added in API level 18
    //
    //public static final String LOCATION_HARDWARE
    //
    description:
      "Allows an application to use location features in hardware, such as the geofencing api.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.MANAGE_DOCUMENTS": {
    //Added in API level 19
    //
    //public static final String MANAGE_DOCUMENTS
    //
    description:
      "Allows an application to manage access to documents, usually as part of a document picker.",
    //
    //This permission should only be requested by the platform document management app. This permission cannot be granted to third-party apps.
  },
  "android.permission.MANAGE_EXTERNAL_STORAGE": {
    //Added in API level 30
    //
    //public static final String MANAGE_EXTERNAL_STORAGE
    //
    description:
      "Allows an application a broad access to external storage in scoped storage. Intended to be used by few apps that need to manage files on behalf of the users.",
    //
    //Protection level: signature|appop|preinstalled
  },
  "android.permission.MANAGE_MEDIA": {
    //Added in API level 31
    //
    //public static final String MANAGE_MEDIA
    //
    //Allows an application to modify and delete media files on this device or any connected storage device without user confirmation. Applications must already be granted the READ_EXTERNAL_STORAGE or MANAGE_EXTERNAL_STORAGE} permissions for this permission to take effect.
    //
    //Even if applications are granted this permission, if applications want to modify or delete media files, they also must get the access by calling MediaStore.createWriteRequest(ContentResolver, Collection), MediaStore.createDeleteRequest(ContentResolver, Collection), or MediaStore.createTrashRequest(ContentResolver, Collection, boolean).
    //
    //This permission doesn't give read or write access directly. It only prevents the user confirmation dialog for these requests.
    //
    //If applications are not granted ACCESS_MEDIA_LOCATION, the system also pops up the user confirmation dialog for the write request.
    //
    //Protection level: signature|appop|preinstalled
  },
  "android.permission.MANAGE_ONGOING_CALLS": {
    //Added in API level 31
    //
    //public static final String MANAGE_ONGOING_CALLS
    //
    description:
      "Allows to query ongoing call details and manage ongoing calls",
    //
    //Protection level: signature|appop
  },
  "android.permission.MANAGE_OWN_CALLS": {
    //Added in API level 26
    //
    //public static final String MANAGE_OWN_CALLS
    //
    description:
      "Allows a calling application which manages its own calls through the self-managed ConnectionService APIs. See PhoneAccount.CAPABILITY_SELF_MANAGED for more information on the self-managed ConnectionService APIs.",
    //
    //Protection level: normal
  },
  "android.permission.MASTER_CLEAR": {
    //Added in API level 1
    //
    //public static final String MASTER_CLEAR
    //
    //Not for use by third-party applications.
  },
  "android.permission.MEDIA_CONTENT_CONTROL": {
    //Added in API level 19
    //
    //public static final String MEDIA_CONTENT_CONTROL
    //
    description:
      "Allows an application to know what content is playing and control its playback.",
    //
    //Not for use by third-party applications due to privacy of media consumption
  },
  "android.permission.MODIFY_AUDIO_SETTINGS": {
    //Added in API level 1
    //
    //public static final String MODIFY_AUDIO_SETTINGS
    //
    description: "Allows an application to modify global audio settings.",
    //
    //Protection level: normal
  },
  "android.permission.MODIFY_PHONE_STATE": {
    //Added in API level 1
    //
    //public static final String MODIFY_PHONE_STATE
    //
    description:
      "Allows modification of the telephony state - power on, mmi, etc. Does not include placing calls.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.MOUNT_FORMAT_FILESYSTEMS": {
    //Added in API level 3
    //
    //public static final String MOUNT_FORMAT_FILESYSTEMS
    //
    description: "Allows formatting file systems for removable storage.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.MOUNT_UNMOUNT_FILESYSTEMS": {
    //Added in API level 1
    //
    //public static final String MOUNT_UNMOUNT_FILESYSTEMS
    //
    description:
      "Allows mounting and unmounting file systems for removable storage.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.NFC": {
    //Added in API level 9
    //
    //public static final String NFC
    //
    description: "Allows applications to perform I/O operations over NFC.",
    //
    //Protection level: normal
  },
  "android.permission.NFC_PREFERRED_PAYMENT_INFO": {
    //Added in API level 30
    //
    //public static final String NFC_PREFERRED_PAYMENT_INFO
    //
    description:
      "Allows applications to receive NFC preferred payment service information.",
    //
    //Protection level: normal
  },
  "android.permission.NFC_TRANSACTION_EVENT": {
    //Added in API level 28
    //
    //public static final String NFC_TRANSACTION_EVENT
    //
    description: "Allows applications to receive NFC transaction events.",
    //
    //Protection level: normal
  },
  "android.permission.PACKAGE_USAGE_STATS": {
    //Added in API level 23
    //
    //public static final String PACKAGE_USAGE_STATS
    //
    //Allows an application to collect component usage statistics
    //
    //Declaring the permission implies intention to use the API and the user of the device can grant permission through the Settings application.
    //
    //Protection level: signature|privileged|development|appop|retailDemo
  },
  "android.permission.PERSISTENT_ACTIVITY": {
    //Added in API level 1
    //Deprecated in API level 15
    //
    //public static final String PERSISTENT_ACTIVITY
    //
    //This constant was deprecated in API level 15.
    //This functionality will be removed in the future; please do not use. Allow an application to make its activities persistent.
  },
  "android.permission.PROCESS_OUTGOING_CALLS": {
    //Added in API level 1
    //Deprecated in API level 29
    //
    //public static final String PROCESS_OUTGOING_CALLS
    //
    //This constant was deprecated in API level 29.
    //Applications should use CallRedirectionService instead of the Intent.ACTION_NEW_OUTGOING_CALL broadcast.
    //
    //Allows an application to see the number being dialed during an outgoing call with the option to redirect the call to a different number or abort the call altogether.
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.QUERY_ALL_PACKAGES": {
    //Added in API level 30
    //
    //public static final String QUERY_ALL_PACKAGES
    //
    description:
      "Allows query of any normal app on the device, regardless of manifest declarations.",
    //
    //Protection level: normal
  },
  "android.permission.READ_CALENDAR": {
    //Added in API level 1
    //
    //public static final String READ_CALENDAR
    //
    description: "Allows an application to read the user's calendar data.",
    //
    //Protection level: dangerous
  },
  "android.permission.READ_CALL_LOG": {
    //Added in API level 16
    //
    //public static final String READ_CALL_LOG
    //
    description: "Allows an application to read the user's call log.",
    //
    //Note: If your app uses the READ_CONTACTS permission and both your minSdkVersion and targetSdkVersion values are set to 15 or lower, the system implicitly grants your app this permission. If you don't need this permission, be sure your targetSdkVersion is 16 or higher.
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.READ_CONTACTS": {
    //Added in API level 1
    //
    //public static final String READ_CONTACTS
    //
    description: "Allows an application to read the user's contacts data.",
    //
    //Protection level: dangerous
  },
  "android.permission.READ_EXTERNAL_STORAGE": {
    //Added in API level 16
    description:
      "Allows an application to read from external storage. \
Any app that declares the WRITE_EXTERNAL_STORAGE permission is implicitly granted this permission.",
    //
    //This permission is enforced starting in API level 19. Before API level 19, this permission is not enforced and all apps still have access to read from external storage. You can test your app with the permission enforced by enabling Protect USB storage under Developer options in the Settings app on a device running Android 4.1 or higher.
    //
    //Also starting in API level 19, this permission is not required to read/write files in your application-specific directories returned by Context.getExternalFilesDir(String) and Context.getExternalCacheDir().
    //
    //Note: If both your minSdkVersion and targetSdkVersion values are set to 3 or lower, the system implicitly grants your app this permission. If you don't need this permission, be sure your targetSdkVersion is 4 or higher.
    //
    //This is a soft restricted permission which cannot be held by an app it its full form until the installer on record whitelists the permission. Specifically, if the permission is allowlisted the holder app can access external storage and the visual and aural media collections while if the permission is not allowlisted the holder app can only access to the visual and aural medial collections. Also the permission is immutably restricted meaning that the allowlist state can be specified only at install time and cannot change until the app is installed. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
    //
    //Protection level: dangerous
  },
  "android.permission.READ_INPUT_STATE": {
    //Added in API level 1
    //Deprecated in API level 16
    //
    //public static final String READ_INPUT_STATE
    //
    //This constant was deprecated in API level 16.
    //The API that used this permission has been removed.
    //
    //Allows an application to retrieve the current state of keys and switches.
    //
    //Not for use by third-party applications.
  },
  "android.permission.READ_LOGS": {
    //Added in API level 1
    //
    //public static final String READ_LOGS
    //
    description:
      "Allows an application to read the low-level system log files.",
    //
    //Not for use by third-party applications, because Log entries can contain the user's private information.
  },
  "android.permission.READ_PHONE_NUMBERS": {
    //Added in API level 26
    //
    //public static final String READ_PHONE_NUMBERS
    //
    description:
      "Allows read access to the device's phone number(s). This is a subset of the capabilities granted by READ_PHONE_STATE but is exposed to instant applications.",
    //
    //Protection level: dangerous
  },
  "android.permission.READ_PHONE_STATE": {
    //Added in API level 1
    //
    //public static final String READ_PHONE_STATE
    //
    description:
      "Allows read only access to phone state, including the current cellular network information, the status of any ongoing calls, and a list of any PhoneAccounts registered on the device.",
    //
    //Note: If both your minSdkVersion and targetSdkVersion values are set to 3 or lower, the system implicitly grants your app this permission. If you don't need this permission, be sure your targetSdkVersion is 4 or higher.
    //
    //Protection level: dangerous
  },
  "android.permission.READ_PRECISE_PHONE_STATE": {
    //Added in API level 30
    //
    //public static final String READ_PRECISE_PHONE_STATE
    //
    description:
      "Allows read only access to precise phone state. Allows reading of detailed information about phone state for special-use applications such as dialers, carrier applications, or ims applications.",
  },
  "android.permission.READ_SMS": {
    //Added in API level 1
    //
    //public static final String READ_SMS
    //
    description: "Allows an application to read SMS messages.",
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.READ_SYNC_SETTINGS": {
    //Added in API level 1
    //
    //public static final String READ_SYNC_SETTINGS
    //
    description: "Allows applications to read the sync settings.",
    //
    //Protection level: normal
  },
  "android.permission.READ_SYNC_STATS": {
    //Added in API level 1
    //
    //public static final String READ_SYNC_STATS
    //
    description: "Allows applications to read the sync stats.",
    //
    //Protection level: normal
  },
  "android.permission.READ_VOICEMAIL": {
    //Added in API level 21
    //
    //public static final String READ_VOICEMAIL
    //
    description: "Allows an application to read voicemails in the system.",
    //
    //Protection level: signature|privileged|role
  },
  "android.permission.REBOOT": {
    //Added in API level 1
    //
    //public static final String REBOOT
    //
    description: "Required to be able to reboot the device.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.RECEIVE_BOOT_COMPLETED": {
    //Added in API level 1
    //
    //public static final String RECEIVE_BOOT_COMPLETED
    //
    description:
      "Allows an application to receive the Intent.ACTION_BOOT_COMPLETED that is broadcast after the system finishes booting. If you don't request this permission, you will not receive the broadcast at that time. Though holding this permission does not have any security implications, it can have a negative impact on the user experience by increasing the amount of time it takes the system to start and allowing applications to have themselves running without the user being aware of them. As such, you must explicitly declare your use of this facility to make that visible to the user.",
    //
    //Protection level: normal
  },
  "android.permission.RECEIVE_MMS": {
    //Added in API level 1
    //
    //public static final String RECEIVE_MMS
    //
    description: "Allows an application to monitor incoming MMS messages.",
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.RECEIVE_SMS": {
    //Added in API level 1
    //
    //public static final String RECEIVE_SMS
    //
    description: "Allows an application to receive SMS messages.",
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.RECEIVE_WAP_PUSH": {
    //Added in API level 1
    //
    //public static final String RECEIVE_WAP_PUSH
    //
    description: "Allows an application to receive WAP push messages.",
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.RECORD_AUDIO": {
    //Added in API level 1
    //
    //public static final String RECORD_AUDIO
    //
    description: "Allows an application to record audio.",
    //
    //Protection level: dangerous
  },
  "android.permission.REORDER_TASKS": {
    //Added in API level 1
    //
    //public static final String REORDER_TASKS
    //
    description: "Allows an application to change the Z-order of tasks.",
    //
    //Protection level: normal
  },
  "android.permission.REQUEST_COMPANION_PROFILE_WATCH": {
    //Added in API level 31
    //
    //public static final String REQUEST_COMPANION_PROFILE_WATCH
    //
    description:
      'Allows app to request to be associated with a device via CompanionDeviceManager as a "watch"',
    //
    //Protection level: normal
  },
  "android.permission.REQUEST_COMPANION_RUN_IN_BACKGROUND": {
    //Added in API level 26
    //
    //public static final String REQUEST_COMPANION_RUN_IN_BACKGROUND
    //
    description:
      "Allows a companion app to run in the background. This permission implies REQUEST_COMPANION_START_FOREGROUND_SERVICES_FROM_BACKGROUND, and allows to start a foreground service from the background. If an app does not have to run in the background, but only needs to start a foreground service from the background, consider using REQUEST_COMPANION_START_FOREGROUND_SERVICES_FROM_BACKGROUND, which is less powerful.",
    //
    //Protection level: normal
  },
  "android.permission.REQUEST_COMPANION_START_FOREGROUND_SERVICES_FROM_BACKGROUND":
    {
      //Added in API level 31
      //
      //public static final String REQUEST_COMPANION_START_FOREGROUND_SERVICES_FROM_BACKGROUND
      //
      description:
        "Allows a companion app to start a foreground service from the background.",
      //
      //Protection level: normal
      //
      //See also:
      //
      //    REQUEST_COMPANION_RUN_IN_BACKGROUND
    },
  "android.permission.REQUEST_COMPANION_USE_DATA_IN_BACKGROUND": {
    //Added in API level 26
    //
    //public static final String REQUEST_COMPANION_USE_DATA_IN_BACKGROUND
    //
    description: "Allows a companion app to use data in the background.",
    //
    //Protection level: normal
  },
  "android.permission.REQUEST_DELETE_PACKAGES": {
    //Added in API level 26
    //
    //public static final String REQUEST_DELETE_PACKAGES
    //
    description:
      "Allows an application to request deleting packages. Apps targeting APIs Build.VERSION_CODES.P or greater must hold this permission in order to use Intent.ACTION_UNINSTALL_PACKAGE or PackageInstaller.uninstall(VersionedPackage, IntentSender).",
    //
    //Protection level: normal
  },
  "android.permission.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS": {
    //Added in API level 23
    //
    //public static final String REQUEST_IGNORE_BATTERY_OPTIMIZATIONS
    //
    description:
      "Permission an application must hold in order to use Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS.",
    //
    //Protection level: normal
  },
  "android.permission.REQUEST_INSTALL_PACKAGES": {
    //Added in API level 23
    //
    //public static final String REQUEST_INSTALL_PACKAGES
    //
    description:
      "Allows an application to request installing packages. Apps targeting APIs greater than 25 must hold this permission in order to use Intent.ACTION_INSTALL_PACKAGE.",
    //
    //Protection level: signature
  },
  "android.permission.REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE": {
    //Added in API level 31
    //
    //public static final String REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE
    //
    description:
      "Allows an application to subscribe to notifications about the presence status change of their associated companion device",
  },
  "android.permission.REQUEST_PASSWORD_COMPLEXITY": {
    //Added in API level 29
    //
    //public static final String REQUEST_PASSWORD_COMPLEXITY
    //
    description:
      "Allows an application to request the screen lock complexity and prompt users to update the screen lock to a certain complexity level.",
    //
    //Protection level: normal
  },
  "android.permission.RESTART_PACKAGES": {
    //Added in API level 1
    //Deprecated in API level 15
    //
    //public static final String RESTART_PACKAGES
    //
    //This constant was deprecated in API level 15.
    //The ActivityManager.restartPackage(String) API is no longer supported.
  },
  "android.permission.SCHEDULE_EXACT_ALARM": {
    //Added in API level 31
    //
    //public static final String SCHEDULE_EXACT_ALARM
    //
    //Allows applications to use exact alarm APIs.
    //
    //Exact alarms should only be used for user-facing features. For more details, see Exact alarm permission.
    //
    //Apps who hold this permission and target API level 31 or above, always stay in the WORKING_SET or lower standby bucket. Applications targeting API level 30 or below do not need this permission to use exact alarm APIs.
  },
  "android.permission.SEND_RESPOND_VIA_MESSAGE": {
    //Added in API level 18
    //
    //public static final String SEND_RESPOND_VIA_MESSAGE
    //
    description:
      "Allows an application (Phone) to send a request to other applications to handle the respond-via-message action during incoming calls.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SEND_SMS": {
    //Added in API level 1
    //
    //public static final String SEND_SMS
    //
    description: "Allows an application to send SMS messages.",
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.SET_ALARM": {
    //Added in API level 9
    //
    //public static final String SET_ALARM
    //
    description:
      "Allows an application to broadcast an Intent to set an alarm for the user.",
    //
    //Protection level: normal
  },
  "android.permission.SET_ALWAYS_FINISH": {
    //Added in API level 1
    //
    //public static final String SET_ALWAYS_FINISH
    //
    description:
      "Allows an application to control whether activities are immediately finished when put in the background.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SET_ANIMATION_SCALE": {
    //Added in API level 1
    //
    //public static final String SET_ANIMATION_SCALE
    //
    description: "Modify the global animation scaling factor.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SET_DEBUG_APP": {
    //Added in API level 1
    //
    //public static final String SET_DEBUG_APP
    //
    description: "Configure an application for debugging.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SET_PREFERRED_APPLICATIONS": {
    //Added in API level 1
    //Deprecated in API level 15
    //
    //public static final String SET_PREFERRED_APPLICATIONS
    //
    //This constant was deprecated in API level 15.
    //No longer useful, see PackageManager.addPackageToPreferred(String) for details.
  },
  "android.permission.SET_PROCESS_LIMIT": {
    //Added in API level 1
    //
    //public static final String SET_PROCESS_LIMIT
    //
    description:
      "Allows an application to set the maximum number of (not needed) application processes that can be running.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SET_TIME": {
    //Added in API level 8
    //
    //public static final String SET_TIME
    //
    description: "Allows applications to set the system time directly.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SET_TIME_ZONE": {
    //Added in API level 1
    //
    //public static final String SET_TIME_ZONE
    //
    description: "Allows applications to set the system time zone directly.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SET_WALLPAPER": {
    //Added in API level 1
    //
    //public static final String SET_WALLPAPER
    //
    description: "Allows applications to set the wallpaper.",
    //
    //Protection level: normal
  },
  "android.permission.SET_WALLPAPER_HINTS": {
    //Added in API level 1
    //
    //public static final String SET_WALLPAPER_HINTS
    //
    description: "Allows applications to set the wallpaper hints.",
    //
    //Protection level: normal
  },
  "android.permission.SIGNAL_PERSISTENT_PROCESSES": {
    //Added in API level 1
    //
    //public static final String SIGNAL_PERSISTENT_PROCESSES
    //
    description:
      "Allow an application to request that a signal be sent to all persistent processes.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SMS_FINANCIAL_TRANSACTIONS": {
    //Added in API level 29
    //Deprecated in API level 31
    //
    //public static final String SMS_FINANCIAL_TRANSACTIONS
    //
    //This constant was deprecated in API level 31.
    //The API that used this permission is no longer functional.
    //
    //Allows financial apps to read filtered sms messages. Protection level: signature|appop
  },
  "android.permission.START_FOREGROUND_SERVICES_FROM_BACKGROUND": {
    //Added in API level 31
    //
    //public static final String START_FOREGROUND_SERVICES_FROM_BACKGROUND
    //
    description:
      "Allows an application to start foreground services from the background at any time. This permission is not for use by third-party applications, with the only exception being if the app is the default SMS app. Otherwise, it's only usable by privileged apps, app verifier app, and apps with any of the EMERGENCY or SYSTEM GALLERY roles.",
  },
  "android.permission.START_VIEW_PERMISSION_USAGE": {
    //Added in API level 29
    //
    //public static final String START_VIEW_PERMISSION_USAGE
    //
    description:
      "Allows the holder to start the permission usage screen for an app.",
    //
    //Protection level: signature|installer
  },
  "android.permission.STATUS_BAR": {
    //Added in API level 1
    //
    //public static final String STATUS_BAR
    //
    description:
      "Allows an application to open, close, or disable the status bar and its icons.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.SYSTEM_ALERT_WINDOW": {
    //Added in API level 1
    //
    //public static final String SYSTEM_ALERT_WINDOW
    //
    description:
      "Allows an app to create windows using the type WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY, shown on top of all other apps. Very few apps should use this permission; these windows are intended for system-level interaction with the user.",
    //
    //Note: If the app targets API level 23 or higher, the app user must explicitly grant this permission to the app through a permission management screen. The app requests the user's approval by sending an intent with action Settings.ACTION_MANAGE_OVERLAY_PERMISSION. The app can check whether it has this authorization by calling Settings.canDrawOverlays().
    //
    //Protection level: signature|setup|appop|installer|pre23|development
  },
  "android.permission.TRANSMIT_IR": {
    //Added in API level 19
    //
    //public static final String TRANSMIT_IR
    //
    description: "Allows using the device's IR transmitter, if available.",
    //
    //Protection level: normal
  },
  "android.permission.UNINSTALL_SHORTCUT": {
    //Added in API level 19
    //
    //public static final String UNINSTALL_SHORTCUT
    //
    //Don't use this permission in your app.
    //This permission is no longer supported.
  },
  "android.permission.UPDATE_DEVICE_STATS": {
    //Added in API level 3
    //
    //public static final String UPDATE_DEVICE_STATS
    //
    description: "Allows an application to update device statistics.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.UPDATE_PACKAGES_WITHOUT_USER_ACTION": {
    //Added in API level 31
    //
    //public static final String UPDATE_PACKAGES_WITHOUT_USER_ACTION
    //
    description:
      "Allows an application to indicate via PackageInstaller.SessionParams.setRequireUserAction(int) that user action should not be required for an app update.",
    //
    //Protection level: normal
  },
  "android.permission.USE_BIOMETRIC": {
    //Added in API level 28
    //
    //public static final String USE_BIOMETRIC
    //
    description: "Allows an app to use device supported biometric modalities.",
    //
    //Protection level: normal
  },
  "android.permission.USE_FINGERPRINT": {
    //Added in API level 23
    //Deprecated in API level 28
    //
    //public static final String USE_FINGERPRINT
    //
    //This constant was deprecated in API level 28.
    //Applications should request USE_BIOMETRIC instead
    //
    //Allows an app to use fingerprint hardware.
    //
    //Protection level: normal
  },
  "android.permission.USE_FULL_SCREEN_INTENT": {
    //Added in API level 29
    //
    //public static final String USE_FULL_SCREEN_INTENT
    //
    description:
      "Required for apps targeting Build.VERSION_CODES.Q that want to use notification full screen intents.",
    //
    //Protection level: normal
  },
  "android.permission.USE_ICC_AUTH_WITH_DEVICE_IDENTIFIER": {
    //Added in API level 31
    //
    //public static final String USE_ICC_AUTH_WITH_DEVICE_IDENTIFIER
    //
    description:
      "Allows to read device identifiers and use ICC based authentication like EAP-AKA. Often required in authentication to access the carrier's server and manage services of the subscriber.",
    //
    //Protection level: signature|appop
  },
  "android.permission.USE_SIP": {
    //Added in API level 9
    //
    //public static final String USE_SIP
    //
    description: "Allows an application to use SIP service.",
    //
    //Protection level: dangerous
  },
  "android.permission.UWB_RANGING": {
    //Added in API level 31
    //
    //public static final String UWB_RANGING
    //
    description:
      "Required to be able to range to devices using ultra-wideband.",
    //
    //Protection level: dangerous
  },
  "android.permission.VIBRATE": {
    //Added in API level 1
    description: "Allows access to the vibrator.",
    //Protection level: normal
  },
  "android.permission.WAKE_LOCK": {
    //Added in API level 1
    //
    //public static final String WAKE_LOCK
    //
    description:
      "Allows using PowerManager WakeLocks to keep processor from sleeping or screen from dimming.",
    //
    //Protection level: normal
  },
  "android.permission.WRITE_APN_SETTINGS": {
    //Added in API level 1
    //
    //public static final String WRITE_APN_SETTINGS
    //
    description:
      "Allows applications to write the apn settings and read sensitive fields of an existing apn settings like user and password.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.WRITE_CALENDAR": {
    //Added in API level 1
    //
    //public static final String WRITE_CALENDAR
    //
    description: "Allows an application to write the user's calendar data.",
    //
    //Protection level: dangerous
  },
  "android.permission.WRITE_CALL_LOG": {
    //Added in API level 16
    //
    //public static final String WRITE_CALL_LOG
    //
    //Allows an application to write (but not read) the user's call log data.
    //
    //Note: If your app uses the WRITE_CONTACTS permission and both your minSdkVersion and targetSdkVersion values are set to 15 or lower, the system implicitly grants your app this permission. If you don't need this permission, be sure your targetSdkVersion is 16 or higher.
    //
    //Protection level: dangerous
    //
    //This is a hard restricted permission which cannot be held by an app until the installer on record whitelists the permission. For more details see PackageInstaller.SessionParams.setWhitelistedRestrictedPermissions(Set).
  },
  "android.permission.WRITE_CONTACTS": {
    //Added in API level 1
    //
    //public static final String WRITE_CONTACTS
    //
    description: "Allows an application to write the user's contacts data.",
    //
    //Protection level: dangerous
  },
  "android.permission.WRITE_EXTERNAL_STORAGE": {
    //Added in API level 4
    //
    //public static final String WRITE_EXTERNAL_STORAGE
    //
    //Allows an application to write to external storage.
    //
    //Note: If both your minSdkVersion and targetSdkVersion values are set to 3 or lower, the system implicitly grants your app this permission. If you don't need this permission, be sure your targetSdkVersion is 4 or higher.
    //
    //Starting in API level 19, this permission is not required to read/write files in your application-specific directories returned by Context.getExternalFilesDir(String) and Context.getExternalCacheDir().
    //
    //If this permission is not allowlisted for an app that targets an API level before Build.VERSION_CODES.Q this permission cannot be granted to apps.
    //
    //Protection level: dangerous
  },
  "android.permission.WRITE_GSERVICES": {
    //Added in API level 1
    //
    //public static final String WRITE_GSERVICES
    //
    description: "Allows an application to modify the Google service map.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.WRITE_SECURE_SETTINGS": {
    //Added in API level 3
    //
    //public static final String WRITE_SECURE_SETTINGS
    //
    description:
      "Allows an application to read or write the secure system settings.",
    //
    //Not for use by third-party applications.
  },
  "android.permission.WRITE_SETTINGS": {
    //Added in API level 1
    //
    //public static final String WRITE_SETTINGS
    //
    description: "Allows an application to read or write the system settings.",
    //
    //Note: If the app targets API level 23 or higher, the app user must explicitly grant this permission to the app through a permission management screen. The app requests the user's approval by sending an intent with action Settings.ACTION_MANAGE_WRITE_SETTINGS. The app can check whether it has this authorization by calling Settings.System.canWrite().
    //
    //Protection level: signature|preinstalled|appop|pre23
  },
  "android.permission.WRITE_SYNC_SETTINGS": {
    //Added in API level 1
    //
    //public static final String WRITE_SYNC_SETTINGS
    //
    description: "Allows applications to write the sync settings.",
    //
    //Protection level: normal
  },
  "android.permission.WRITE_VOICEMAIL": {
    //Added in API level 21
    //
    //public static final String WRITE_VOICEMAIL
    //
    description:
      "Allows an application to modify and remove existing voicemails in the system.",
    //
    //Protection level: signature|privileged|role
  },
};
