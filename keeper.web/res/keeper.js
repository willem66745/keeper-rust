// Show loader and disable the rest of the user interface
function show_loader() {
    $("#foo").addClass('ui-disabled');
    $.mobile.loading("show", {
        text: "loading",
        textVisible: true
    });
}

// Hide loader and activate the rest of the user interface
function hide_loader() {
    $.mobile.loading("hide");
    $("#foo").removeClass('ui-disabled');
}

// Retrieve all switch configurations and update web user interface
function load_switches() {
    show_loader();

    $.post("/api/switches", function(data) {
        var items = data.length;

        $("#content").empty();
        $("#content").hide();

        $.each(data, function(index, object) {
            var li = $("#content").append('<li class="ui-field-contain"><a href="#details">'
                    + '<h2>' + object + '</h2>'
                    + '<p><select name="flip_' + object + '" id="flip_' + object + '" data-role="slider" data-mini="true">'
                        + '<option value="off">Off</option>'
                        + '<option value="on">On</option>'
                    + '</select></p>'
                    + '<p class="ui-li-aside"></p>'
                    + '</a></li>');

            li = li.find("li").last();
            var flip = li.find("#flip_" + object);
            flip.slider().change(function(event) {
                li.addClass('ui-disabled');
                $.post("/api/switch/" + object + "/" + $(this).val(), function(data) {
                    flip.val(data ? "on" : "off").slider("refresh");
                }).always(function() {
                    li.removeClass('ui-disabled');
                });
            });

            $.ajax({
                url: "/api/get/" + object,
                type: "POST",
                async: false,
                timeout: 2000
            }).done(function(data) {
                if (data.switch == true) {
                    flip.val("on").slider("refresh");
                } else {
                    flip.val("off").slider("refresh");
                }

                // update side part of LI item
                var next_event = Object.keys(data.next_events).shift();
                var aside = li.find("p.ui-li-aside")
                if (next_event === undefined) {
                    aside.html('manual');
                } else {
                    var next_state = data.next_events[next_event];
                    next_event = new Date(Date.parse(next_event));
                    aside.html("next: "
                            + next_event.getHours() + ":" + next_event.getMinutes()
                            + " <strong>" + (next_state ? "on" : "off") + "</strong>");
                }
            }).fail(function() {
            }).always(function() {
                items--;
                if (items == 0) {
                    hide_loader();
                }
            });
        });

        $("#content").listview("refresh");
        $("#content").show();
    });
}

// Load web page and set event handlers
$(function() {
    load_switches();

    $("#refresh").click(function(event) {
        event.preventDefault();
        load_switches();
    });
});
