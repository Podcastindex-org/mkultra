//Msg types
//---------
// -2 Noop
// -1 Room welcome
//  1 Regular message
//  3 User room entry announce

$(document).ready(function () {
    let hostName = location.hostname;
    let hostScheme = window.location.protocol;
    let messages = $('div.mesgs');
    let inbox = messages.find('div.msg_history');
    let isEphemeral = $('input#isEphemeral');
    let appIconUrlBase = 'https://podcastindex.org/api/images/';
    let pewAudioFile = '/pew.mp3';
    let pewAudio = new Audio(pewAudioFile);
    const urlParams = new URLSearchParams(window.location.search);
    const chat_id = urlParams.get('cid');
    var intvlChatPolling = null;
    var connection = null;
    var lgHasSentMetadata = false;
    var lgNostrWaitCycles = 0;
    var lgLoggedInWithNostr = false;
    var lgUsedNostrBrowserExtension = false;

    //Connect to some nostr relays
    const nostrRelayPool = new NostrTools.SimplePool();
    let nostrRelays = [
        'wss://relay.mostr.pub',
        'wss://nostr.wine',
        'wss://welcome.nostr.wine',
        'wss://relay.current.fyi',
        'wss://relay.damus.io',
        'wss://relay.snort.social',
        'wss://relay.nostr.info',
        'wss://nos.lol',
        'wss://eden.nostr.land',
        'wss://brb.io',
        'wss://offchain.pub',
        'wss://nostr.orangepill.dev'
    ];

    //Get or build a user identity
    $('button.msg_submit').prop('disabled', true);
    var userProfile = localStorage.getItem('userProfile');
    if (userProfile === null) {
        var userProfileObject = {};
        userProfile = {};

        bootbox.dialog({
            className: "identityBox",
            title: "Who do you want to be?",
            closeButton: false,
            message: '<form class="bootbox-form">' +
                '<input class="bootbox-input form-control imageUpload" disabled>' +
                '<input class="bootbox-input bootbox-input-text form-control" type="text" placeholder="Display Name">' +
                '<div style="clear:both">&nbsp;</div>' +
                '<input class="imageUploadFilePicker" type="file">' +
                '<div class="loginProfileDisplay"><span class="loginAbout"></span></div>' +
                '<br>' +
                '<input class="bootbox-input bootbox-input-text form-control nostrPrivateKeyInput" ' +
                'type="text" ' +
                'placeholder="nsec1... paste nostr private key here">' +
                '</form>',
            buttons: {
                Create: {
                    disabled: "false",
                    label: "Create",
                    className: "btn-primary createButton",
                    callback: function () {
                        //Get what's in the text box
                        var username = $('div.bootbox div.modal-body .bootbox-form .bootbox-input-text').val();

                        //Client side val
                        if (username.trim() == "") {
                            console.log("No username given.");
                            return false;
                        }

                        userProfileObject.id = userProfile.pubkey || generateId(63);
                        userProfileObject.pubkey = userProfile.pubkey || "";
                        userProfileObject.privkey = userProfile.privkey || "";
                        userProfileObject.created_at = Date.now() / 1000;
                        userProfileObject.name = userProfile.name || username;
                        userProfileObject.display_name = userProfile.display_name || username;
                        userProfileObject.about = userProfile.about || "";
                        userProfileObject.picture = userProfile.picture || "";
                        userProfileObject.banner = userProfile.banner || "";
                        userProfileObject.website = userProfile.website || "";

                        //If there isn't a pubkey in the user profile object, create one
                        //and show it to the user so they can save it
                        if (userProfileObject.privkey === "" && !lgUsedNostrBrowserExtension) {
                            userProfileObject.privkey = NostrTools.generatePrivateKey();
                        }
                        if (userProfileObject.pubkey === "") {
                            userProfileObject.pubkey = NostrTools.getPublicKey(userProfileObject.privkey);
                        }
                        userProfileObject.id = userProfileObject.pubkey;

                        //See if the user wants their profile sent to Nostr
                        if (!lgLoggedInWithNostr) {
                            bootbox.confirm({
                                title: 'Identity created!',
                                message: '<p>Your new identity has been created and save in your browser. Hang on to these' +
                                    ' keys in case you ever need to recover it:</p>' +
                                    '<ul class="popupKeyDisplay">' +
                                    '<li>Public: ' + NostrTools.nip19.npubEncode(userProfileObject.pubkey) + '</li>' +
                                    '<li>Private: ' + NostrTools.nip19.nsecEncode(userProfileObject.privkey) + '</li>' +
                                    '</ul>' +
                                    '<p>Do you want to inform the Nostr relay network of your new identity so you can take ' +
                                    'it with you to other places?</p>',
                                buttons: {
                                    cancel: {
                                        label: '<i class="fa fa-times"></i> No'
                                    },
                                    confirm: {
                                        label: '<i class="fa fa-check"></i> Yes',
                                        className: 'nostrConnect'
                                    }
                                },
                                callback: function (result) {
                                    console.log('This was logged in the callback: ' + result);
                                    if (result) {
                                        postNostrMetadata(userProfileObject, nostrRelayPool, nostrRelays).await;
                                    }
                                }
                            });
                        } else {
                            if ($('div.bootbox input#loginAnnounceNostr').is(':checked')) {
                                console.log("Announce room to Nostr");
                                postNostrMessage(
                                    userProfileObject,
                                    "I just joined the live stream chat at: " +
                                    hostScheme + "//" + hostName + "/?cid=" + chat_id,
                                    nostrRelayPool,
                                    nostrRelays);
                            }
                        }

                        //Show the identity keys on the page
                        displayIdentity(userProfileObject);

                        //Save the identity we are using
                        localStorage.setItem('userProfile', JSON.stringify(userProfileObject));
                        userProfile = userProfileObject;
                        $('button.msg_submit').prop('disabled', false);

                        connectWebSocket("[" + truncateUsername(userProfile.display_name) + " just entered the room]");

                        return true;
                    }
                },
                Nostr: {
                    label: "Nostr",
                    className: "nostrConnect",
                    callback: function () {
                        $('div.bootbox').find('button').prop('disabled', true);
                        $('div.bootbox').find('button.nostrConnect').empty().html('<i class="fa fa-spin fa-spinner">');
                        $('div.bootbox').find('.modal-footer').prepend(
                            '<span class="loadingMessage">Checking for browser extension...</span>'
                        );
                        waitForNostr();
                        return false;
                    }
                }
            }
        });

        $(document).on('change', 'input.imageUploadFilePicker', function () {
            let imagedata = encodeImageFileAsURL($(this)[0]);
        });
        $(document).on('change keyup', 'input.nostrPrivateKeyInput', function () {
            let enteredString = $(this).val();
            if (enteredString.indexOf("nsec") === 0) {
                console.log("Private key manual input", enteredString);
                let {type, data} = NostrTools.nip19.decode(enteredString);
                console.log("Decoded Nsec key", type, data);
                console.log("Re-encoded private key", NostrTools.nip19.nsecEncode(data));
                userProfile.privkey = data;
                userProfile.pubkey = NostrTools.getPublicKey(data);
                getNostrPubkey(nostrRelayPool, nostrRelays);
                lgUsedNostrBrowserExtension = false;
                $(this).hide();
            }
        });
    } else {
        userProfile = JSON.parse(userProfile);
        getNostrPubkey(nostrRelayPool, nostrRelays);
        displayIdentity(userProfile);
        $('button.msg_submit').prop('disabled', false);
    }

    function encodeImageFileAsURL(element) {
        var file = element.files[0];
        var reader = new FileReader();
        reader.onloadend = function () {
            userProfile.picture = reader.result;
            $('div.bootbox div.modal-body div.bootbox-body .bootbox-form input.imageUpload')
                .css('background-image', 'url(' + userProfile.picture + ')')
                .addClass('resolved');
        }
        reader.readAsDataURL(file);
    }

    //Wait for window.nostr to show up by looping and checking
    //if the object becomes defined
    function waitForNostr() {
        console.log("Wait for nostr");
        lgNostrWaitCycles++;
        if (typeof window.nostr !== "undefined") {
            //variable exists, do what you want
            getNostrPubkey(nostrRelayPool, nostrRelays);
            lgUsedNostrBrowserExtension = true;
        } else {
            if (lgNostrWaitCycles > 8) {
                allowPrivateKeyInput();
            } else {
                setTimeout(waitForNostr, 250);
            }
        }
    }

    //Nostr browser extension isn't installed so drop back to private key entry
    function allowPrivateKeyInput() {
        $('div.bootbox').find('button.nostrConnect').empty().text('Nostr');
        $('div.bootbox div.modal-body div.bootbox-body .bootbox-form .bootbox-input.nostrPrivateKeyInput').show();
        $('div.bootbox').find('.modal-footer span.loadingMessage').html(
            'Not found. <a href="https://chrome.google.com/webstore/detail/nos2x/kpgefcfmnafjgpblomihpgmejjdanjjp">Get one</a>?'
        );
        $('div.bootbox').find('button').prop('disabled', false);
    }

    //Send a set_metadata event to the nostr relays for this new identity
    async function postNostrMetadata(profileData, pool, relays) {
        nostrMetadata = {};
        nostrMetadata.name = profileData.name;
        nostrMetadata.display_name = profileData.display_name;
        nostrMetadata.about = profileData.about;
        nostrMetadata.picture = profileData.picture;
        nostrMetadata.banner = profileData.banner;
        nostrMetadata.website = profileData.website;

        let event = {
            id: 0,
            kind: 0,
            created_at: Math.floor(Date.now() / 1000),
            tags: [],
            content: JSON.stringify(nostrMetadata),
            pubkey: profileData.pubkey,
            "sig": ""
        }

        event.id = NostrTools.getEventHash(event)
        event = await signNostrEvent(event, profileData);

        let pubs = pool.publish(relays, event)
        pubs.on('ok', () => {
            console.log(`relay has accepted our set_metadata event`)
            lgHasSentMetadata = true;
        })
        pubs.on('failed', reason => {
            console.log(`failed to publish set_metadata to relay: ${reason}`)
        })
    }

    async function getNostrPubkey(pool, relays) {
        if (typeof userProfile.pubkey === "undefined" || userProfile.pubkey == "") {
            userProfile.pubkey = await window.nostr.getPublicKey();
        }
        $('div.bootbox').find('button.nostrConnect').empty().html('<i class="fa fa-spin fa-spinner">');
        console.log(userProfile.pubkey);

        let sub = pool.sub(
            relays,
            [
                {
                    authors: [
                        userProfile.pubkey
                    ],
                    kinds: [0]
                }
            ]
        )
        sub.on('event', event => {
            console.log('Got the profile metadata:', event);

            //Hang on to the user profile data
            nostrUserProfile = JSON.parse(event.content);
            console.log("Nostr Profile", nostrUserProfile);
            userProfile.name = nostrUserProfile.name.trim();
            userProfile.about = nostrUserProfile.about.trim();
            userProfile.picture = nostrUserProfile.picture.trim();

            //Update the displayed identity keys
            displayIdentity(userProfile);

            //If the identity modal dialog is currently on-screen, take appropriate action
            //to fill in it's details with this user's nostr info
            if ($('div.bootbox .bootbox-form').length > 0) {
                let nameToShow = nostrUserProfile.display_name || userProfile.name || "";
                $('div.bootbox .bootbox-form .bootbox-input-text').val(nameToShow);
                $('div.bootbox .bootbox-form input.imageUpload')
                    .css('background-image', 'url(' + userProfile.picture + ')')
                    .addClass('resolved');
                $('div.bootbox .bootbox-form div.loginProfileDisplay .loginAbout').text(userProfile.about);

                //Expose the nostr announcement option if it isn't already visible
                if ($('div.bootbox div.modal-footer div.loginAnnounceChatToNostr').length === 0) {
                    $('div.bootbox div.modal-footer span.loadingMessage').remove();
                    $('div.bootbox div.modal-footer').prepend(
                        '<div class="loginAnnounceChatToNostr">' +
                        '  <label for="loginAnnounceNostr">Announce this chat to your Nostr feed?</label>' +
                        '  <input id="loginAnnounceNostr" type="checkbox">' +
                        '</div>'
                    );
                }

                $('div.bootbox').find('button.nostrConnect').remove();
                $('div.bootbox').find('input.imageUploadFilePicker').remove();
                $('div.bootbox').find('button.createButton').text("Login");
                $('div.bootbox').find('button').prop('disabled', false);
            }
            lgLoggedInWithNostr = true;
        })
        let events = await pool.list(relays, [{kinds: [0]}]);
        let event = await pool.get(relays, {authors: [userProfile.pubkey]});
    }

    //Send a new post to nostr
    async function postNostrMessage(profileData, message, pool, relays) {
        nostrMetadata = {};
        nostrMetadata.name = profileData.name;
        nostrMetadata.display_name = profileData.display_name;
        nostrMetadata.about = profileData.about;
        nostrMetadata.picture = profileData.picture;
        nostrMetadata.banner = profileData.banner;
        nostrMetadata.website = profileData.website;

        let event = {
            id: 0,
            pubkey: profileData.pubkey,
            created_at: Math.floor(Date.now() / 1000),
            kind: 1,
            tags: [],
            content: message + "\n\n#testing #podcasting20 #mkultra",
            sig: ""
        }

        event.id = NostrTools.getEventHash(event);
        event = await signNostrEvent(event, profileData);

        let pubs = pool.publish(relays, event);
        pubs.on('ok', (relay) => {
            console.log(`relay: [${relay}] has accepted our message posting event`);
        })
        pubs.on('failed', reason => {
            console.log(`failed to publish message to relay: ${reason}`);
        })
    }

    //_via: https://stackoverflow.com/questions/1349404/generate-random-string-characters-in-javascript?answertab=votes#tab-top
    // dec2hex :: Integer -> String
    // i.e. 0-255 -> '00'-'ff'
    function dec2hex(dec) {
        return dec.toString(16).padStart(2, "0");
    }

    // generateId :: Integer -> String
    function generateId(len) {
        var arr = new Uint8Array((len || 40) / 2)
        window.crypto.getRandomValues(arr)
        return Array.from(arr, dec2hex).join('')
    }

    //Submit message
    $('textarea.msg_text').keyup(function (event) {
        if (event.keyCode === 13) {
            if (event.shiftKey === false) {
                $('button.msg_submit').click();
            }
        }
    });
    $(document).on('click', 'button.msg_submit', function () {
        //Get the message text
        let messageText = $('textarea.msg_text').val();
        //Bail if the message text is empty
        if (messageText.trim() == "") {
            return false;
        }
        //Try sending to Nostr if the ephemeral box isn't checked
        var sendToNostr = false;
        if (!isEphemeral.is(":checked")) {
            sendToNostr = true;
        }
        submitChatMessage(messageText, sendToNostr, 1);

        $('textarea.msg_text').val('');
    });


    async function submitChatMessage(textMessage, withNostr, msgType) {
        let lastMsgId = $('div.outgoing_msg:last').data('msgid');
        if (typeof lastMsgId === "undefined") {
            lastMsgId = 0;
        }

        //Get a correct name to use
        var postingName = userProfile.display_name || userProfile.name;

        let event = {
            "id": lastMsgId,
            "created_at": Math.trunc(Date.now() / 1000),
            "pubkey": userProfile.pubkey,
            "user_name": postingName.trim().substring(0, 32),
            "content": textMessage,
            "tags": [],
            "chat_id": chat_id,
            "picture": userProfile.picture,
            "kind": msgType,
            "sig": ""
        };

        event.id = NostrTools.getEventHash(event);
        console.log("Event hash", event.id);

        //Sign the event
        event = await signNostrEvent(event, userProfile);

        //Fix the event after signing
        delete event.tags;
        event.id = lastMsgId;

        //Send the event to the websocket
        connection.send(JSON.stringify(event));

        //Only send posts to Nostr when the ephemeral control checkbox is unchecked
        if (withNostr) {
            if (!lgHasSentMetadata) {
                postNostrMetadata(userProfile, nostrRelayPool, nostrRelays).await;
            }
            postNostrMessage(userProfile, textMessage, nostrRelayPool, nostrRelays);
        }
    }

    //Websocket testing
    window.WebSocket = window.WebSocket || window.MozWebSocket;

    wsConnector = setInterval(function () {
        //console.log("Checking websocket...");
        //console.log(connection);
        if (connection === null || connection.readyState === 3) {
            connectWebSocket();
        }
    }, 3000);

    function connectWebSocket(announce) {
        connection = new WebSocket('wss://' + window.location.hostname + ':8443');
        var connectingSpan = document.getElementById("connecting");
        console.log("Connecting...");

        var postEnterMessage = announce || false;

        connection.onopen = function () {
            //connectingSpan.style.display =
            pollForMessages(connection);
            console.log("Websocket connected.");
            if (postEnterMessage !== false && userProfile.id !== "") {
                submitChatMessage(postEnterMessage, false, 3);
            }
        };
        connection.onerror = function (error) {
            //connectingSpan.innerHTML = "Error occured";
            console.log("Websocket error.");
        };
        connection.onmessage = function (message) {
            var data = JSON.parse(message.data);
            var someNew = false;

            if (Array.isArray(data)) {
                data.forEach((element, index) => {
                    if ($('div.outgoing_msg[data-msgid=' + element.id + ']').length == 0) {
                        if (element.pubkey !== userProfile.pubkey) {
                            someNew = true;
                        }
                        let message = element.content || "";
                        let msgtype = element.kind || 1;
                        let userName = element.user_name || "";
                        let userPicture = element.picture || "";
                        let dateTime = new Date(element.created_at * 1000).toISOString();
                        var msgClassName = "";

                        //If this is the top welcome post
                        if (msgtype == -1) {
                            msgClassName = "welcome";
                        }

                        //Call out our own messages a bit visually different
                        var fixedUserName = truncateUsername(userName);
                        var nameDisplay = '      <h5>' + fixedUserName + ':</h5>&nbsp;';
                        if (element.pubkey == userProfile.pubkey) {
                            msgClassName += " self_msg";
                            var nameDisplay = "";
                        }

                        //Add some modifier classes to the message text if needed
                        var messageClasses = "";

                        //Write the message to the screen
                        inbox.append('' +
                            '<div class="outgoing_msg message msgtype' + msgtype + ' ' + msgClassName + '" data-msgid="' + element.id + '">' +
                            '  <div class="sent_msg">' +
                            '    <div class="outgoing_msg_img">' +
                            '      <img src="' + userPicture + '">' +
                            '    </div>' +
                            '    <div class="sent_withd_msg">' +
                            nameDisplay +
                            '      <span class="time_date" data-timestamp="' + dateTime + '">' +
                            prettyDate(dateTime) + '' +
                            '      </span>' +
                            '      <span class="messageText' + messageClasses + '">' + message + '</span>' +
                            '    </div>' +
                            '  </div>' +
                            '</div>');

                        inbox.scrollTop(900719925474099);
                    }
                });
            }

            if (someNew) {
                //pewAudio.play();
            }
        };
        connection.onclose = function () {
            console.log("Websocket closed.");
        };

        pollForMessages = function (conn) {
            let lastMsgId = $('div.outgoing_msg:last').data('msgid');
            if (typeof lastMsgId === "undefined") {
                lastMsgId = 0;
            }
            // console.log(lastMsgId);

            let event = {
                "id": lastMsgId,
                "created_at": Math.trunc(Date.now() / 1000),
                "pubkey": "",
                "user_name": "",
                "content": "",
                "tags": [],
                "chat_id": chat_id,
                "picture": "",
                "kind": -2,
                "sig": ""
            };

            conn.send(JSON.stringify(event));
        };
    }

    connectWebSocket();

    //Show the identity of the logged in user at the bottom
    function displayIdentity(profileObject) {
        if (typeof profileObject.pubkey === "string" && profileObject.pubkey != "") {
            $('div.identity_display span.pubkey').text(
                NostrTools.nip19.npubEncode(profileObject.pubkey)
            );
        }
        if (typeof profileObject.privkey === "string" && profileObject.privkey != "") {
            $('div.identity_display span.privkey').text(
                NostrTools.nip19.nsecEncode(profileObject.privkey)
            );
        }
        if (typeof profileObject.picture === "string" && profileObject.picture != "") {
            $('img.userAvatarHeader').attr('src', profileObject.picture);
        }
    }

    //Update timestamps
    function updateTimeStamps() {
        let timestamps = $('div.message span.time_date');

        timestamps.each(function (idx) {
            let timestamp = $(this).data('timestamp');
            $(this).html(prettyDate(timestamp));
        });
    }

    //The profile edit modal
    $(document).on('click', 'a#editProfile', function () {
        bootbox.confirm({
            title: 'Logout?',
            message: 'Would you like to log out of your current identity? Be sure you saved your keys.',
            buttons: {
                cancel: {
                    label: '<i class="fa fa-times"></i> No'
                },
                confirm: {
                    label: '<i class="fa fa-check"></i> Yes',
                    className: 'btn-danger'
                }
            },
            callback: function (result) {
                console.log('This was logged in the callback: ' + result);
                if (result) {
                    localStorage.removeItem('userProfile');
                    location.reload();
                }
            }
        });
    });

    function truncateUsername(userName) {
        var fixedUserName = userName.trim();
        if (fixedUserName.length > 32) {
            fixedUserName = fixedUserName.substring(0, 29) + "...";
        }
        return fixedUserName;
    }

    async function signNostrEvent(event, profileData) {
        console.log("Event", event);
        var signedEvent = {};

        if (typeof window.nostr !== "undefined") {
            console.log(await window.nostr.getPublicKey())
            signedEvent = await window.nostr.signEvent(event);
        } else {
            signedEvent = NostrTools.signEvent(event, profileData.privkey)
        }

        console.log("Signed event", signedEvent);

        return signedEvent;
    }

    setInterval(function () {
        updateTimeStamps();
    }, 59000);
});