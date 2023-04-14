$(document).ready(function () {
    let messages = $('div.mesgs');
    let inbox = messages.find('div.msg_history');
    let appIconUrlBase = 'https://podcastindex.org/api/images/';
    let pewAudioFile = '/pew.mp3';
    let pewAudio = new Audio(pewAudioFile);
    const urlParams = new URLSearchParams(window.location.search);
    const chat_id = urlParams.get('cid');
    var intvlChatPolling = null;
    var connection = null;

    //Connect to some nostr relays
    const nostrRelayPool = new NostrTools.SimplePool();
    let nostrRelays = [
        'wss://relay.damus.io',
        'wss://relay.snort.social',
        'wss://relay.nostr.info',
        'wss://nos.lol',
        'wss://eden.nostr.land'
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
                '<input class="bootbox-input form-control imageUpload">' +
                '<input class="bootbox-input bootbox-input-text form-control" type="text" placeholder="Display Name">' +
                '<div style="clear:both">&nbsp;</div>' +
                '<input class="imageUploadFilePicker" type="file">' +
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
                        if(username.trim() == "") {
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
                        //and show it to the user so they can
                        userProfileObject.privkey = NostrTools.generatePrivateKey();
                        userProfileObject.pubkey = NostrTools.getPublicKey(userProfileObject.privkey);
                        postNostrMetadata(userProfileObject, nostrRelayPool, nostrRelays).await;

                        displayIdentity(userProfileObject);

                        //Save the identity we are using
                        localStorage.setItem('userProfile', JSON.stringify(userProfileObject));
                        userProfile = userProfileObject;
                        $('button.msg_submit').prop('disabled', false);

                        connectWebSocket("Just entered the room.");

                        return true;
                    }
                },
                Nostr: {
                    label: "Nostr",
                    className: "nostrConnect",
                    callback: function () {
                        waitForNostr();
                        return false;
                    }
                }
            }
        });

        $(document).on('click', 'input.imageUpload', function() {
            $('input.imageUpload').click();
        });
        $(document).on('change', 'input.imageUploadFilePicker', function() {
            let imagedata = encodeImageFileAsURL($(this)[0]);
        });
    } else {
        userProfile = JSON.parse(userProfile);
        displayIdentity(userProfile);
        $('button.msg_submit').prop('disabled', false);
    }

    function encodeImageFileAsURL(element) {
        var file = element.files[0];
        var reader = new FileReader();
        reader.onloadend = function() {
            userProfile.picture = reader.result;
            $('div.bootbox div.modal-body div.bootbox-body .bootbox-form input.imageUpload')
                .css('background-image', 'url('+userProfile.picture+')')
                .addClass('resolved');
        }
        reader.readAsDataURL(file);
    }

    //Wait for window.nostr to show up by looping and checking
    //if the object becomes defined
    function waitForNostr() {
        console.log("Wait for nostr");
        if (typeof window.nostr !== "undefined") {
            //variable exists, do what you want
            getNostrPubkey(nostrRelayPool, nostrRelays);
        } else {
            setTimeout(waitForNostr, 250);
        }
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
            kind: 0,
            created_at: Math.floor(Date.now() / 1000),
            tags: [],
            content: JSON.stringify(nostrMetadata),
            pubkey: profileData.pubkey
        }

        event.id = NostrTools.getEventHash(event)
        event.sig = NostrTools.signEvent(event, profileData.privkey)

        let pubs = pool.publish(relays, event)
        pubs.on('ok', () => {
            console.log(`relay has accepted our set_metadat event`)
        })
        pubs.on('failed', reason => {
            console.log(`failed to publish set_metadata to relay: ${reason}`)
        })
    }

    async function getNostrPubkey(pool, relays) {
        userProfile.pubkey = await window.nostr.getPublicKey();
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
            userProfile.name = nostrUserProfile.name;
            userProfile.about = nostrUserProfile.about;
            userProfile.picture = nostrUserProfile.picture;

            //If the identity modal dialog is currently on-screen, take appropriate action
            //to fill in it's details with this user's nostr info
            if ($('div.bootbox div.modal-body div.bootbox-body .bootbox-form .bootbox-input').length > 0) {
                $('div.bootbox div.modal-body div.bootbox-body .bootbox-form .bootbox-input-text').val(userProfile.name);
            }
            if ($('div.bootbox div.modal-body div.bootbox-body .bootbox-form').length > 0) {
                $('div.bootbox div.modal-body div.bootbox-body .bootbox-form input.imageUpload')
                    .css('background-image', 'url('+userProfile.picture+')')
                    .addClass('resolved');
            }
            $('div.bootbox').find('button.nostrConnect').remove();
            $('div.bootbox').find('input.imageUploadFilePicker').remove();
            $('div.bootbox').find('button.createButton').text("Login");
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
            kind: 1,
            created_at: Math.floor(Date.now() / 1000),
            tags: [],
            content: message + "#mkutesting",
            pubkey: profileData.pubkey
        }

        event.id = NostrTools.getEventHash(event)
        event.sig = NostrTools.signEvent(event, profileData.privkey)

        let pubs = pool.publish(relays, event)
        pubs.on('ok', () => {
            console.log(`relay has accepted our message posting event`)
        })
        pubs.on('failed', reason => {
            console.log(`failed to publish message to relay: ${reason}`)
        })
    }

    //_via: https://stackoverflow.com/questions/1349404/generate-random-string-characters-in-javascript?answertab=votes#tab-top
    // dec2hex :: Integer -> String
    // i.e. 0-255 -> '00'-'ff'
    function dec2hex(dec) {
        return dec.toString(16).padStart(2, "0")
    }

    // generateId :: Integer -> String
    function generateId(len) {
        var arr = new Uint8Array((len || 40) / 2)
        window.crypto.getRandomValues(arr)
        return Array.from(arr, dec2hex).join('')
    }

    //Submit message
    $('textarea.msg_text').keyup(function(event) {
        if (event.keyCode === 13) {
            if (event.shiftKey === false) {
                $('button.msg_submit').click();
            }
        }
    });
    $(document).on('click', 'button.msg_submit', function () {
        submitChatMessage();
    });


    function submitChatMessage(textMessage, withNostr) {
        let msgText = textMessage || $('textarea.msg_text').val();
        let lastMsgId = $('div.outgoing_msg:last').data('msgid');
        if (typeof lastMsgId === "undefined") {
            lastMsgId = 0;
        }
        //console.log(lastMsgId);

        let params = {
            "id": lastMsgId,
            "time": Math.trunc(Date.now() / 1000),
            "user_id": userProfile.id,
            "user_name": userProfile.name,
            "comment": msgText,
            "chat_id": chat_id,
            "picture": userProfile.picture
        };

        connection.send(JSON.stringify(params));

        postNostrMessage(userProfile, msgText, nostrRelayPool, nostrRelays);

        $('textarea.msg_text').val('');

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
            if(postEnterMessage !== false) {
                submitChatMessage(postEnterMessage);
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
                        if (element.user_id !== userProfile.id) {
                            someNew = true;
                        }
                        let message = element.comment || "";
                        let userName = element.user_name || "";
                        let userPicture = element.picture || "";
                        let dateTime = new Date(element.time * 1000).toISOString();
                        var msgClassName = "";

                        //If this is the top welcome post
                        if(element.id == 1 && element.user_name == "" && element.comment == "Welcome!") {
                            msgClassName = "welcome";
                        }

                        //Call out our own messages a bit visually different
                        if(element.user_id == userProfile.id) {
                            msgClassName += " self_msg";
                        }

                        //Write the message to the screen
                        inbox.append('' +
                            '<div class="outgoing_msg message '+msgClassName+'" data-msgid="' + element.id + '">' +
                            '<div class="sent_msg"><div class="outgoing_msg_img"><img src="'+userPicture+'"></div>' +
                            '<div class="sent_withd_msg"><h5>' + userName + '</h5>' +
                            '<span class="time_date" data-timestamp="' + dateTime + '">' +
                            prettyDate(dateTime) + '</span>' +
                            '<p>' + message + '</p>' +
                            '</div></div></div>');
                        inbox.animate({scrollTop: 999999999});
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

            let params = {
                "id": lastMsgId,
                "time": Math.trunc(Date.now() / 1000),
                "user_id": userProfile.id,
                "user_name": userProfile.name,
                "comment": "",
                "chat_id": chat_id,
                "picture": userProfile.picture
            };

            conn.send(JSON.stringify(params));
        };
    }

    connectWebSocket();

    //Show the identity of the logged in user at the bottom
    function displayIdentity(profileObject) {
        if(userProfile.pubkey != "") {
            $('div.identity_display span.pubkey').text(
                NostrTools.nip19.npubEncode(profileObject.pubkey)
            );
        }
        if(userProfile.privkey != "") {
            $('div.identity_display span.privkey').text(
                NostrTools.nip19.nsecEncode(profileObject.privkey)
            );
        }
        $('img.userAvatarHeader').attr('src', profileObject.picture);
    }

    //Update timestamps
    function updateTimeStamps() {
        let timestamps = $('div.message span.time_date');

        timestamps.each(function (idx) {
            let timestamp = $(this).data('timestamp');
            $(this).html(prettyDate(timestamp));
        });
    }

    setInterval(function () {
        updateTimeStamps();
    }, 59000);
});